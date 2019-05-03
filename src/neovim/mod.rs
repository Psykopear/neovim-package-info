use crate::consts;
use crate::parser::{parse_cargo_toml, parse_package_json, parse_pipfile, parse_piplock};
use crate::store::{Cratesio, Npm, Pypi, Store};

use neovim_lib::{Neovim, NeovimApi, Session, Value};
use rayon::prelude::*;
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
            let mut chunks: Vec<Value> = messages
                .iter()
                .map(|(message, highlight)| {
                    vec![
                        Value::from(message.to_string()),
                        Value::from(highlight.to_string()),
                    ]
                    .into()
                })
                .collect();
            chunks.insert(
                0,
                vec![Value::from(consts::PREFIX), Value::from(consts::GREY_HG)].into(),
            );
            match buffer.set_virtual_text(&mut self.nvim, 0, line_number, chunks, vec![]) {
                Ok(_) => (),
                Err(error) => self.echo(&format!("{}", error)),
            }
        }
    }

    fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    fn handle_cargo_toml(&mut self, content: &str) {
        let cargo_toml = parse_cargo_toml(&content).expect("Can't parse cargo toml");

        // Concatenate all dependencie so we can parallelize network calls
        let dependencies = cargo_toml
            .dependencies
            .iter()
            .chain(cargo_toml.dev_dependencies.iter())
            .chain(cargo_toml.build_dependencies.iter());

        // First find the line number of each requirement and set a
        // waiting message as virtual text
        for dep in dependencies {
            let mut line_number = 0;
            for (index, line) in content.split("\n").enumerate() {
                if line.to_string().starts_with(&format!("{} = ", dep.0)) {
                    line_number = index
                }
            }
            self.set_text(
                &vec![("...".to_string(), consts::GREY_HG.to_string())],
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
                    self.cratesio.check_dependency(&name, &dependency.req()),
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

    fn handle_pipfile(&mut self, content: &str, lockfile_content: &str) {
        let pipfile = parse_pipfile(&content).expect("Error parsing pipfile");
        let lockfile = parse_piplock(&lockfile_content).expect("Error parsing lockfile");

        // Concatenate all dependencie so we can parallelize network calls
        let dependencies = pipfile
            .dependencies
            .iter()
            .chain(pipfile.dev_dependencies.iter());

        // First find the line number of each requirement and set a
        // waiting message as virtual text
        for dep in dependencies {
            let mut line_number = 0;
            for (index, line) in content.split("\n").enumerate() {
                if line.to_string().starts_with(&format!("{} = ", dep.0))
                    || line.to_string().starts_with(&format!("\"{}\" = ", dep.0))
                {
                    line_number = index
                }
            }
            // Parse lockfile
            let lockdata = if lockfile.default.contains_key(dep.0) {
                lockfile.default.get(dep.0)
            } else if lockfile.develop.contains_key(dep.0) {
                lockfile.develop.get(dep.0)
            } else {
                None
            };
            match lockdata {
                Some(package_info) => {
                    self.set_text(
                        &vec![
                            (
                                format!("{} ", package_info["version"].as_str().expect("")),
                                consts::GREY_HG.to_string(),
                            ),
                            ("...".to_string(), consts::GREY_HG.to_string()),
                        ],
                        line_number as i64,
                    );
                }
                None => {
                    self.set_text(
                        &vec![("...".to_string(), consts::GREY_HG.to_string())],
                        line_number as i64,
                    );
                }
            }
        }

        let dependencies: Vec<(String, Vec<(String, String)>)> = pipfile
            .dependencies
            .par_iter()
            .chain(pipfile.dev_dependencies.par_iter())
            .map(|(name, dependency)| {
                let req = format!("{:?}", dependency);
                (name.to_string(), self.pypi.check_dependency(&name, &req))
            })
            .collect();
        for (name, messages) in dependencies {
            let mut line_number = 0;
            for (index, line) in content.split("\n").enumerate() {
                if line.to_string().starts_with(&format!("{} = ", name))
                    || line.to_string().starts_with(&format!("\"{}\" = ", name))
                {
                    line_number = index
                }
            }

            // Parse lockfile
            let lockdata = if lockfile.default.contains_key(&name) {
                lockfile.default.get(&name)
            } else if lockfile.develop.contains_key(&name) {
                lockfile.develop.get(&name)
            } else {
                None
            };
            match lockdata {
                Some(package_info) => {
                    let mut current = package_info["version"].as_str().expect("").chars();
                    current.next();
                    current.next();
                    let mut res = vec![(
                        format!("current: {}  latest: ", current.as_str()),
                        consts::GREY_HG.to_string(),
                    )];
                    res.append(&mut messages.clone());
                    self.set_text(&res, line_number as i64);
                }
                None => {
                    self.set_text(&messages, line_number as i64);
                }
            }
        }
    }

    fn handle_package_json(&mut self, content: &str) {
        let package_json = parse_package_json(&content).expect("Can't parse package json");

        // Concatenate all dependencie so we can parallelize network calls
        let dependencies = package_json
            .dependencies
            .iter()
            .chain(package_json.dev_dependencies.iter());

        // First find the line number of each requirement and set a
        // waiting message as virtual text
        for dep in dependencies {
            let mut line_number = 0;
            for (index, line) in content.split("\n").enumerate() {
                if line.to_string().contains(&format!("\"{}\": \"", dep.0)) {
                    line_number = index
                }
            }
            self.set_text(
                &vec![("...".to_string(), consts::GREY_HG.to_string())],
                line_number as i64,
            );
        }

        let dependencies: Vec<(String, Vec<(String, String)>)> = package_json
            .dependencies
            .par_iter()
            .chain(package_json.dev_dependencies.par_iter())
            .map(|(name, dependency)| {
                (
                    name.to_string(),
                    self.npm.check_dependency(&name, &dependency),
                )
            })
            .collect();
        for (name, messages) in dependencies {
            let mut line_number = 0;
            for (index, line) in content.split("\n").enumerate() {
                if line.to_string().contains(&format!("\"{}\": \"", name)) {
                    line_number = index
                }
            }
            self.set_text(&messages, line_number as i64);
        }
    }

    fn recv(&mut self) {
        let receiver = self.nvim.session.start_event_loop_channel();

        for (event, args) in receiver {
            let file_path = &parse_string(&args[0]).expect("File path not received!");
            let content = fs::read_to_string(file_path).expect("Can't read to string");
            match Messages::from(event) {
                Messages::CargoToml => {
                    self.handle_cargo_toml(&content);
                }
                Messages::Pipfile => {
                    // Parse lock file
                    let lockfile_path = format!("{}.lock", file_path);
                    let lockfile_content =
                        fs::read_to_string(lockfile_path).expect("Can't read lockfile to string");
                    self.handle_pipfile(&content, &lockfile_content);
                }
                Messages::PackageJson => {
                    self.handle_package_json(&content);
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
