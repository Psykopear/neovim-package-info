mod package_json;
mod pipfile;

use crate::consts;
use crate::neovim::DependencyInfo;
use cargo_toml;
use failure::Error;
use std::collections::HashMap;

pub struct Manifest {
    dependencies: Vec<String>,
}

impl From<cargo_toml::Manifest> for Manifest {
    fn from(cargo_toml_manifest: cargo_toml::Manifest) -> Manifest {
        let dependencies: Vec<String> = cargo_toml_manifest
            .dependencies
            .iter()
            .chain(cargo_toml_manifest.dev_dependencies.iter())
            .chain(cargo_toml_manifest.build_dependencies.iter())
            .map(|(name, _)| name.to_string())
            .collect();
        Manifest { dependencies }
    }
}

pub struct Lockfile {
    dependencies: HashMap<String, String>,
}

impl From<HashMap<String, toml::Value>> for Lockfile {
    fn from(cargo_lock: HashMap<String, toml::Value>) -> Lockfile {
        let packages: Vec<toml::Value> =
            if let Ok(packages) = cargo_lock["package"].clone().try_into() {
                packages
            } else {
                vec![]
            };
        let dependencies: HashMap<_, _> = packages
            .iter()
            .map(|p| {
                (
                    p["name"].as_str().expect("").to_string(),
                    p["version"].as_str().expect("").to_string(),
                )
            })
            .collect();
        Lockfile { dependencies }
    }
}

pub trait Parser {
    fn new(manifest_content: &str, lockfile_content: &str) -> Self;
    fn get_dependencies(&self) -> Result<Vec<DependencyInfo>, Error>;
    fn parse_manifest(&self) -> Result<Manifest, Error>;
    fn parse_lockfile(&self) -> Result<Lockfile, Error>;
}

pub struct CargoParser {
    manifest_content: String,
    lockfile_content: String,
}

impl Parser for CargoParser {
    fn new(manifest_content: &str, lockfile_content: &str) -> Self {
        CargoParser {
            manifest_content: manifest_content.to_string(),
            lockfile_content: lockfile_content.to_string(),
        }
    }

    fn parse_manifest(&self) -> Result<Manifest, Error> {
        Ok(cargo_toml::Manifest::from_str(&self.manifest_content)?.into())
    }

    fn parse_lockfile(&self) -> Result<Lockfile, Error> {
        let cargo_lock: HashMap<String, toml::Value> = toml::from_str(&self.lockfile_content)?;
        Ok(cargo_lock.into())
    }

    fn get_dependencies(&self) -> Result<Vec<DependencyInfo>, Error> {
        let cargo_toml = self.parse_manifest()?;
        let cargo_lock = self.parse_lockfile()?;

        // Concatenate all dependencie so we can parallelize network calls
        Ok(cargo_toml
            .dependencies
            .iter()
            .map(|name| {
                let mut line_number: i64 = 0;
                for (index, line) in self.manifest_content.split("\n").enumerate() {
                    if line.to_string().starts_with(&format!("{} = ", name)) {
                        line_number = index as i64
                    }
                }
                if let Some(version) = cargo_lock.dependencies.get(name) {
                    DependencyInfo {
                        line_number,
                        name: name.to_string(),
                        current: version.to_string(),
                        latest: vec![("...".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        name: name.to_string(),
                        current: "--".to_string(),
                        latest: vec![("...".to_string(), consts::GREY_HG.to_string())],
                        line_number,
                    }
                }
            })
            .collect())
    }
}

pub fn parse_pipfile(content: &str) -> Result<pipfile::Pipfile, Error> {
    Ok(pipfile::Pipfile::from_str(content)?)
}

pub fn parse_piplock(content: &str) -> Result<pipfile::Piplock, Error> {
    Ok(pipfile::Piplock::from_str(content)?)
}

pub fn parse_package_json(content: &str) -> Result<package_json::PackageJson, Error> {
    Ok(package_json::PackageJson::from_str(content)?)
}
