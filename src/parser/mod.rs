mod cargo_parser;
mod package_json;
mod pipfile;

pub use cargo_parser::CargoParser;
pub use package_json::PackageJsonParser;
pub use pipfile::PipfileParser;

use crate::neovim::DependencyInfo;
use failure::Error;
use std::collections::HashMap;

pub struct Manifest {
    dependencies: Vec<(String, String)>,
}

pub struct Lockfile {
    dependencies: HashMap<String, String>,
}

pub trait Parser {
    fn get_dependencies(
        manifest_content: &str,
        lockfile_content: &str,
    ) -> Result<Vec<DependencyInfo>, Error>;
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error>;
    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error>;
}
