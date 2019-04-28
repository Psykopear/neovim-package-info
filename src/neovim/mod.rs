use crate::fetcher::{Cratesio, Npm, Pypi, Store};
use cargo_toml::{Dependency, Manifest};
use neovim_lib::{Neovim, NeovimApi, Session, Value};
use rayon::prelude::*;
use semver::{Version, VersionReq};
use std::fs;

pub fn parse_string(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .ok_or("cannot parse error".to_owned())
        .map(|s| String::from(s))
}

enum Messages {
    CargoToml,
    Pipfile,
    PackageJson,
    Unknown(String),
}

impl From<String> for Messages {
    fn from(event: String) -> Self {
        match &event[..] {
            "cargo-toml" => Messages::CargoToml,
            "pipfile" => Messages::Pipfile,
            "package-json" => Messages::PackageJson,
            _ => Messages::Unknown(event),
        }
    }
}

static PREFIX: &str = "  ¤ ";

struct EventHandler {
    nvim: Neovim,
    cratesio: Cratesio,
    npm: Npm,
    pypi: Pypi,
}

impl EventHandler {
    fn new() -> Self {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);
        let cratesio = Cratesio::new();
        let pypi = Pypi::new();
        let npm = Npm::new();
        EventHandler {
            nvim,
            cratesio,
            pypi,
            npm,
        }
    }

    fn set_text(&mut self, messages: &Vec<(String, String)>, line_number: i64) {
        if let Ok(buffer) = self.nvim.get_current_buf() {
            let chunks: Vec<Value> = messages
                .iter()
                .map(|(message, highlight)| {
                    vec![
                        Value::from(message.to_string()),
                        Value::from(highlight.to_string()),
                    ]
                    .into()
                })
                .collect();
            match buffer.set_virtual_text(&mut self.nvim, 0, line_number, chunks, vec![]) {
                Ok(_) => (),
                Err(error) => self.echo(&format!("{}", error)),
            }
        }
    }

    fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    fn check_dependency<T: Store>(
        &self,
        name: &str,
        dependency: &Dependency,
        store: &T,
    ) -> Vec<(String, String)> {
        if let Ok(store_version) = store.get_max_version(name) {
            if let Ok(latest_version) = Version::parse(&store_version) {
                if let Ok(requirement) = Version::parse(dependency.req()) {
                    if latest_version.major > requirement.major {
                        vec![
                            (PREFIX.to_string(), "Comment".to_string()),
                            (format!("{}", latest_version), "Error".to_string()),
                        ]
                    } else if latest_version.minor > requirement.minor {
                        let split: Vec<String> = latest_version
                            .to_string()
                            .split('.')
                            .map(|x| x.to_string())
                            .collect();
                        vec![
                            (PREFIX.to_string(), "Comment".to_string()),
                            (format!("{}.", split[0]).to_string(), "Comment".to_string()),
                            (split[1..].join("."), "Number".to_string()),
                        ]
                    } else if latest_version.patch > requirement.patch {
                        let split: Vec<String> = latest_version
                            .to_string()
                            .split('.')
                            .map(|x| x.to_string())
                            .collect();
                        vec![
                            (PREFIX.to_string(), "Comment".to_string()),
                            (
                                format!("{}.", split[..2].join(".")).to_string(),
                                "Comment".to_string(),
                            ),
                            (split[2..].join("."), "String".to_string()),
                        ]
                    } else {
                        vec![(
                            format!("{}{}", PREFIX, latest_version),
                            "Comment".to_string(),
                        )]
                    }
                } else {
                    if let Ok(requirement) = VersionReq::parse(dependency.req()) {
                        if requirement.matches(&latest_version) {
                            vec![
                                (PREFIX.to_string(), "Comment".to_string()),
                                (format!("{}", latest_version), "String".to_string()),
                            ]
                        } else {
                            vec![
                                (PREFIX.to_string(), "Comment".to_string()),
                                (format!("{}", latest_version), "Number".to_string()),
                            ]
                        }
                    } else {
                        vec![(
                            format!("{}{}", PREFIX, latest_version),
                            "Comment".to_string(),
                        )]
                    }
                }
            } else {
                vec![(
                    format!("{}Error parsing store version {}", PREFIX, store_version),
                    "Comment".to_string(),
                )]
            }
        } else {
            vec![(
                format!("{}Error getting store version for {}", PREFIX, name),
                "Comment".to_string(),
            )]
        }
    }

    fn recv(&mut self) {
        let receiver = self.nvim.session.start_event_loop_channel();

        for (event, args) in receiver {
            match Messages::from(event) {
                Messages::CargoToml => {
                    let file_path = &parse_string(&args[0]).expect("File path not received!");
                    let content = fs::read_to_string(file_path).expect("Can't read to string");
                    let cargo_toml = Manifest::from_str(&content).expect("Can't parse cargo toml");

                    let dependencies = cargo_toml
                        .dependencies
                        .iter()
                        .chain(cargo_toml.dev_dependencies.iter())
                        .chain(cargo_toml.build_dependencies.iter());
                    for dep in dependencies {
                        let mut line_number = 0;
                        for (index, line) in content.split("\n").enumerate() {
                            if line.to_string().starts_with(&format!("{} = ", dep.0)) {
                                line_number = index
                            }
                        }
                        self.set_text(
                            &vec![(format!("{}...", PREFIX), "Comment".to_string())],
                            line_number as i64,
                        );
                    }

                    let dependencies: Vec<(String, Vec<(String, String)>)> = cargo_toml
                        .dependencies
                        .par_iter()
                        .chain(cargo_toml.dev_dependencies.par_iter())
                        .chain(cargo_toml.build_dependencies.par_iter())
                        .map(|(name, dependency)| {
                            (
                                name.to_string(),
                                self.check_dependency(&name, &dependency, &self.cratesio),
                            )
                        })
                        .collect();
                    for (name, messages) in dependencies {
                        let mut line_number = 0;
                        for (index, line) in content.split("\n").enumerate() {
                            if line.to_string().starts_with(&format!("{} = ", name)) {
                                line_number = index
                            }
                        }
                        self.set_text(&messages, line_number as i64);
                    }
                }
                Messages::Pipfile => {
                    let file_path = &parse_string(&args[0]).expect("File path not received!");
                    self.echo(file_path);
                }
                Messages::PackageJson => {
                    let file_path = &parse_string(&args[0]).expect("File path not received!");
                    self.echo(file_path);
                }
                Messages::Unknown(event) => {
                    self.echo(&format!("Unkown command: {}, args: {:?}", event, args));
                }
            }
        }
    }
}

pub fn run() {
    let mut event_handler = EventHandler::new();
    event_handler.recv();
}
