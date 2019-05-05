use crate::consts;
use crate::parser::{CargoParser, PackageJsonParser, Parser, PipfileParser};
use crate::store::{Cratesio, Npm, Pypi, Store};
use failure::Error;
use neovim_lib::{Neovim, NeovimApi, Session, Value};
use rayon::prelude::*;
use std::fs;

pub struct DependencyInfo {
    pub name: String,
    pub current: String,
    pub latest: Vec<(String, String)>,
    pub line_number: i64,
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

struct NeovimSession {
    pub nvim: Neovim,
    pub buffer_number: i64,
}

impl NeovimSession {
    pub fn new() -> Self {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);
        NeovimSession {
            nvim,
            buffer_number: 0,
        }
    }

    pub fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    pub fn set_text(&mut self, messages: &Vec<(String, String)>, line_number: i64) {
        // First search the buffer
        let buffers = self.nvim.list_bufs().expect("Error listing buffers");
        let mut buffer = None;
        for buf in buffers {
            if buf
                .get_number(&mut self.nvim)
                .expect("Error getting buffer number")
                == self.buffer_number
            {
                buffer = Some(buf)
            }
        }
        if let Some(buffer) = buffer {
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

    pub fn start_event_loop_channel(&mut self) -> std::sync::mpsc::Receiver<(String, Vec<Value>)> {
        self.nvim.session.start_event_loop_channel()
    }
}

struct EventHandler {
    cratesio: Cratesio,
    npm: Npm,
    pypi: Pypi,
}

impl EventHandler {
    fn new() -> Self {
        let cratesio = Cratesio::new();
        let pypi = Pypi::new();
        let npm = Npm::new();
        EventHandler {
            cratesio,
            pypi,
            npm,
        }
    }

    fn handle_cargo_toml(
        &self,
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        let cargo_parser = CargoParser::new(&content, &lockfile_content);
        let dependencies: Vec<DependencyInfo> = cargo_parser.get_dependencies()?;
        self.handle_generic(&dependencies, nvim_session)?;
        let latest_dependencies = dependencies
            .par_iter()
            .map(|dep| DependencyInfo {
                current: dep.current.clone(),
                line_number: dep.line_number,
                name: dep.name.clone(),
                latest: self.cratesio.check_dependency(&dep),
            })
            .collect();
        self.handle_generic(&latest_dependencies, nvim_session)?;
        Ok(())
    }

    fn handle_pipfile(
        &self,
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        let pipfile_parser = PipfileParser::new(&content, &lockfile_content);
        let dependencies: Vec<DependencyInfo> = pipfile_parser.get_dependencies()?;
        self.handle_generic(&dependencies, nvim_session)?;
        let latest_dependencies = dependencies
            .par_iter()
            .map(|dep| DependencyInfo {
                current: dep.current.clone(),
                line_number: dep.line_number,
                name: dep.name.clone(),
                latest: self.pypi.check_dependency(&dep),
            })
            .collect();
        self.handle_generic(&latest_dependencies, nvim_session)?;
        Ok(())
    }

    fn handle_package_json(
        &self,
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        let package_json_parser = PackageJsonParser::new(&content, &lockfile_content);
        let dependencies: Vec<DependencyInfo> = package_json_parser.get_dependencies()?;
        self.handle_generic(&dependencies, nvim_session)?;
        let latest_dependencies = dependencies
            .par_iter()
            .map(|dep| DependencyInfo {
                current: dep.current.clone(),
                line_number: dep.line_number,
                name: dep.name.clone(),
                latest: self.npm.check_dependency(&dep),
            })
            .collect();
        self.handle_generic(&latest_dependencies, nvim_session)?;

        Ok(())
    }

    fn handle_generic(
        &self,
        dependencies: &Vec<DependencyInfo>,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        for dep in dependencies {
            let mut lines = vec![(dep.current.to_string(), consts::GREY_HG.to_string())];
            lines.append(&mut dep.latest.clone());
            nvim_session.set_text(&lines, dep.line_number);
        }
        Ok(())
    }

    fn recv(&self, nvim_session: &mut NeovimSession) {
        let receiver = nvim_session.start_event_loop_channel();

        for (event, args) in receiver {
            if let Some(buffer_number) = args[1].as_i64() {
                nvim_session.buffer_number = buffer_number;
                if let Some(file_path) = args[0].as_str() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        match Messages::from(event) {
                            Messages::CargoToml => {
                                if let Ok(lockfile_content) =
                                    fs::read_to_string(file_path.replace(".toml", ".lock"))
                                {
                                    match self.handle_cargo_toml(
                                        &content,
                                        &lockfile_content,
                                        nvim_session,
                                    ) {
                                        Ok(_) => (),
                                        Err(error) => {
                                            nvim_session.echo(&error.to_string());
                                        }
                                    };
                                } else {
                                    nvim_session.echo("Can't find lockfile");
                                }
                            }
                            Messages::Pipfile => {
                                // Parse lock file
                                if let Ok(lockfile_content) =
                                    fs::read_to_string(format!("{}.lock", file_path))
                                {
                                    match self.handle_pipfile(
                                        &content,
                                        &lockfile_content,
                                        nvim_session,
                                    ) {
                                        Ok(_) => (),
                                        Err(error) => {
                                            nvim_session.echo(&error.to_string());
                                        }
                                    };
                                } else {
                                    nvim_session.echo("Can't find lockfile!");
                                }
                            }
                            Messages::PackageJson => {
                                if let Ok(lockfile_content) = fs::read_to_string(
                                    file_path.replace("package.json", "yarn.lock"),
                                ) {
                                    match self.handle_package_json(
                                        &content,
                                        &lockfile_content,
                                        nvim_session,
                                    ) {
                                        Ok(_) => (),
                                        Err(error) => {
                                            nvim_session.echo(&error.to_string());
                                        }
                                    };
                                } else {
                                    nvim_session.echo("Can't find lockfile");
                                }
                            }
                            Messages::Unknown(event) => {
                                nvim_session
                                    .echo(&format!("Unkown command: {}, args: {:?}", event, args));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run() {
    let mut nvim_session = NeovimSession::new();
    let event_handler = EventHandler::new();
    event_handler.recv(&mut nvim_session);
}
