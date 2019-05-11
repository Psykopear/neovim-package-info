use crate::cache::Cache;
use crate::consts;
use crate::parser::{CargoParser, PackageJsonParser, Parser, PipfileParser};
use crate::store::{Cratesio, Npm, Pypi, Store};
use failure::Error;
use neovim_lib::neovim_api::Buffer;
use neovim_lib::{Neovim, NeovimApi, Session, Value};
use rayon::prelude::*;
use semver;
use std::fs;

pub struct DependencyInfo {
    pub name: String,
    pub requirement: String,
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

    fn get_buffer(&mut self) -> Option<Buffer> {
        let buffers = self.nvim.list_bufs().expect("Error listing buffers");
        for buf in buffers {
            if buf
                .get_number(&mut self.nvim)
                .expect("Error getting buffer number")
                == self.buffer_number
            {
                return Some(buf);
            }
        }
        None
    }

    pub fn set_text(&mut self, messages: &Vec<(String, String)>, line_number: i64) {
        if let Some(buffer) = self.get_buffer() {
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

struct EventHandler;

impl EventHandler {
    fn handle_cargo_toml(
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
        mut cache: &mut Cache,
    ) -> Result<(), Error> {
        let dependencies: Vec<DependencyInfo> =
            CargoParser::get_dependencies(&content, &lockfile_content)?;
        Self::handle_generic(&dependencies, &mut cache, nvim_session, Cratesio);
        Ok(())
    }

    fn handle_pipfile(
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
        mut cache: &mut Cache,
    ) -> Result<(), Error> {
        let dependencies: Vec<DependencyInfo> =
            PipfileParser::get_dependencies(&content, &lockfile_content)?;
        Self::handle_generic(&dependencies, &mut cache, nvim_session, Pypi);
        Ok(())
    }

    fn handle_package_json(
        content: &str,
        lockfile_content: &str,
        nvim_session: &mut NeovimSession,
        mut cache: &mut Cache,
    ) -> Result<(), Error> {
        let dependencies: Vec<DependencyInfo> =
            PackageJsonParser::get_dependencies(&content, &lockfile_content)?;
        Self::handle_generic(&dependencies, &mut cache, nvim_session, Npm);
        Ok(())
    }

    fn handle_generic<T: Store>(
        dependencies: &Vec<DependencyInfo>,
        cache: &mut Cache,
        nvim_session: &mut NeovimSession,
        _: T,
    ) {
        let dependencies = dependencies
            .par_iter()
            .map(|dep| DependencyInfo {
                requirement: dep.requirement.clone(),
                current: dep.current.clone(),
                line_number: dep.line_number,
                name: dep.name.clone(),
                latest: cache.get(&dep, &<T as Store>::check_dependency),
            })
            .collect();
        cache.update(&dependencies);
        for dep in dependencies {
            let mut lines: Vec<(String, String)> = vec![];
            match semver::VersionReq::parse(&dep.requirement) {
                Ok(requirement) => {
                    let current = match semver::Version::parse(&dep.current) {
                        Ok(current) => current,
                        _ => continue,
                    };
                    if requirement.matches(&current) {
                        lines.append(&mut vec![(
                            dep.current.to_string(),
                            consts::GREY_HG.to_string(),
                        )])
                    } else {
                        lines.append(&mut vec![(
                            dep.current.to_string(),
                            consts::RED_HG.to_string(),
                        )]);
                    }
                }
                _ => {
                    lines.append(&mut vec![(
                        dep.current.to_string(),
                        consts::GREY_HG.to_string(),
                    )]);
                }
            };
            lines.append(&mut dep.latest.clone());
            nvim_session.set_text(&lines, dep.line_number);
        }
    }

    fn recv(nvim_session: &mut NeovimSession) {
        let receiver = nvim_session.start_event_loop_channel();
        let mut cargo_cache: Cache = Cache::new(30);
        let mut pypi_cache: Cache = Cache::new(30);
        let mut npm_cache: Cache = Cache::new(30);

        for (event, args) in receiver {
            nvim_session.buffer_number = match args[1].as_i64() {
                Some(number) => number,
                _ => continue,
            };
            let file_path = match args[0].as_str() {
                Some(file_path) => file_path,
                _ => continue,
            };
            let manifest_content = match fs::read_to_string(&file_path) {
                Ok(content) => content,
                _ => continue,
            };
            match Messages::from(event) {
                Messages::CargoToml => {
                    let lockfile_content = fs::read_to_string(file_path.replace(".toml", ".lock"))
                        .unwrap_or("".to_string());
                    match Self::handle_cargo_toml(
                        &manifest_content,
                        &lockfile_content,
                        nvim_session,
                        &mut cargo_cache,
                    ) {
                        Ok(_) => (),
                        Err(error) => {
                            nvim_session.echo(&error.to_string());
                        }
                    };
                }
                Messages::Pipfile => {
                    // Parse lock file, or use an empty string
                    let lockfile_content =
                        fs::read_to_string(format!("{}.lock", file_path)).unwrap_or("".to_string());
                    match Self::handle_pipfile(
                        &manifest_content,
                        &lockfile_content,
                        nvim_session,
                        &mut pypi_cache,
                    ) {
                        Ok(_) => (),
                        Err(error) => {
                            nvim_session.echo(&error.to_string());
                        }
                    };
                }
                Messages::PackageJson => {
                    let lockfile_content =
                        fs::read_to_string(file_path.replace("package.json", "yarn.lock"))
                            .unwrap_or("".to_string());
                    match Self::handle_package_json(
                        &manifest_content,
                        &lockfile_content,
                        nvim_session,
                        &mut npm_cache,
                    ) {
                        Ok(_) => (),
                        Err(error) => {
                            nvim_session.echo(&error.to_string());
                        }
                    };
                }
                Messages::Unknown(event) => {
                    nvim_session.echo(&format!("Unkown command: {}, args: {:?}", event, args));
                }
            }
        }
    }
}

pub fn run() {
    let mut nvim_session = NeovimSession::new();
    EventHandler::recv(&mut nvim_session);
}
