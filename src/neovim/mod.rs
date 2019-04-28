use crate::parser::parse_cargo_toml;
use crate::store::{Cratesio, Npm, Pypi, Store};

use neovim_lib::{Neovim, NeovimApi, Session, Value};
use rayon::prelude::*;
use std::fs;

static PREFIX: &str = "  -> ";

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
                vec![
                    Value::from(format!("{}", PREFIX)),
                    Value::from("Comment".to_string()),
                ]
                .into(),
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

    fn recv(&mut self) {
        let receiver = self.nvim.session.start_event_loop_channel();

        for (event, args) in receiver {
            match Messages::from(event) {
                Messages::CargoToml => {
                    let file_path = &parse_string(&args[0]).expect("File path not received!");
                    let content = fs::read_to_string(file_path).expect("Can't read to string");
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
                            &vec![("...".to_string(), "Comment".to_string())],
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
