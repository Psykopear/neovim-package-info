use crate::fetcher::{Cratesio, Npm, Pypi, Store};
use cargo_toml::{Dependency, Manifest};
use neovim_lib::{Neovim, NeovimApi, Session, Value};
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

    fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    fn echoerr(&mut self, message: &str) {
        self.nvim
            .command(&format!("echoerr \"{}\"", message))
            .unwrap();
    }

    fn check_dependency<T: Store>(
        &self,
        name: &str,
        dependency: &Dependency,
        store: &T,
    ) -> Result<String, Box<std::error::Error>> {
        let requirement = VersionReq::parse(dependency.req())?;
        let latest_version = store.get_max_version(name)?;
        let latest_version = Version::parse(&latest_version)?;
        if requirement.matches(&latest_version) {
            Ok(format!("{} matches {}", name, requirement))
        } else {
            Ok(format!("{} does not match {}", name, requirement))
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
                    for (name, dependency) in cargo_toml.dependencies {
                        let res = self
                            .check_dependency(&name, &dependency, &self.cratesio)
                            .expect("Error checking dependency");
                        self.echo(&res);
                    }
                    for (name, dependency) in cargo_toml.dev_dependencies {
                        let res = self
                            .check_dependency(&name, &dependency, &self.cratesio)
                            .expect("Error checking dependency");
                        self.echo(&res);
                    }
                    for (name, dependency) in cargo_toml.build_dependencies {
                        let res = self
                            .check_dependency(&name, &dependency, &self.cratesio)
                            .expect("Error checking dependency");
                        self.echo(&res);
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
                    self.echoerr(&format!("Unkown command: {}, args: {:?}", event, args));
                }
            }
        }
    }
}

pub fn run() {
    let mut event_handler = EventHandler::new();
    event_handler.recv();
}
