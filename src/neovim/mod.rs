use crate::consts;
use crate::parser::{parse_package_json, parse_pipfile, parse_piplock, CargoParser, Parser};
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
}

impl NeovimSession {
    pub fn new() -> Self {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);
        NeovimSession { nvim }
    }

    pub fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    pub fn set_text(&mut self, messages: &Vec<(String, String)>, line_number: i64) {
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
                latest: self.cratesio.check_dependency(&dep.name, &dep.current),
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
        let pipfile = parse_pipfile(&content)?;
        let lockfile = parse_piplock(&lockfile_content)?;

        // First write data about current versions and a loader
        let dependencies: Vec<DependencyInfo> = pipfile
            .dependencies
            .iter()
            .chain(pipfile.dev_dependencies.iter())
            .map(|(name, _)| {
                let mut line_number: i64 = 0;
                for (index, line) in content.split("\n").enumerate() {
                    if line.to_string().starts_with(&format!("{} = ", name))
                        || line.to_string().starts_with(&format!("\"{}\" = ", name))
                    {
                        line_number = index as i64
                    }
                }
                // Parse lockfile
                let lockdata = if lockfile.default.contains_key(name) {
                    lockfile.default.get(name)
                } else if lockfile.develop.contains_key(name) {
                    lockfile.develop.get(name)
                } else {
                    None
                };
                match lockdata {
                    Some(package_info) => DependencyInfo {
                        line_number,
                        name: name.to_string(),
                        current: package_info["version"]
                            .as_str()
                            .expect("Version missing")
                            .to_string(),
                        latest: vec![("...".to_string(), consts::GREY_HG.to_string())],
                    },
                    None => DependencyInfo {
                        name: name.to_string(),
                        current: "--".to_string(),
                        latest: vec![("...".to_string(), consts::GREY_HG.to_string())],
                        line_number,
                    },
                }
            })
            .collect();
        self.handle_generic(&dependencies, nvim_session)?;

        // Then concurrently fetch latest data and write it
        let latest_dependencies = dependencies
            .par_iter()
            .map(|dep| DependencyInfo {
                current: dep.current.clone(),
                line_number: dep.line_number,
                name: dep.name.clone(),
                latest: self.pypi.check_dependency(&dep.name, &dep.current),
            })
            .collect();
        self.handle_generic(&latest_dependencies, nvim_session)?;

        Ok(())
    }

    fn handle_package_json(
        &self,
        content: &str,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        let package_json = parse_package_json(&content)?;

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
            nvim_session.set_text(
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
            nvim_session.set_text(&messages, line_number as i64);
        }
        Ok(())
    }

    fn handle_generic(
        &self,
        dependencies: &Vec<DependencyInfo>,
        nvim_session: &mut NeovimSession,
    ) -> Result<(), Error> {
        for dep in dependencies {
            let mut lines = vec![(
                format!("current: {}, latest: ", dep.current),
                consts::GREY_HG.to_string(),
            )];
            lines.append(&mut dep.latest.clone());
            nvim_session.set_text(&lines, dep.line_number);
        }
        Ok(())
    }

    fn recv(&self, nvim_session: &mut NeovimSession) {
        let receiver = nvim_session.start_event_loop_channel();

        for (event, args) in receiver {
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
                                match self.handle_pipfile(&content, &lockfile_content, nvim_session)
                                {
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
                            match self.handle_package_json(&content, nvim_session) {
                                Ok(_) => (),
                                Err(error) => {
                                    nvim_session.echo(&error.to_string());
                                }
                            };
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

pub fn run() {
    let mut nvim_session = NeovimSession::new();
    let event_handler = EventHandler::new();
    event_handler.recv(&mut nvim_session);
}
