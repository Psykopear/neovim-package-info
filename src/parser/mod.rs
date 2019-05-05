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

impl From<pipfile::Pipfile> for Manifest {
    fn from(pipfile: pipfile::Pipfile) -> Manifest {
        let dependencies: Vec<String> = pipfile
            .dependencies
            .iter()
            .chain(pipfile.dev_dependencies.iter())
            .map(|(name, _)| name.to_string())
            .collect();
        Manifest { dependencies }
    }
}

impl From<package_json::PackageJson> for Manifest {
    fn from(package_json: package_json::PackageJson) -> Manifest {
        let dependencies: Vec<String> = package_json
            .dependencies
            .iter()
            .chain(package_json.dev_dependencies.iter())
            .map(|(name, _)| name.to_string())
            .collect();
        Manifest { dependencies }
    }
}

pub struct Lockfile {
    dependencies: HashMap<String, String>,
}

impl From<pipfile::Piplock> for Lockfile {
    fn from(piplock: pipfile::Piplock) -> Lockfile {
        let dependencies: HashMap<String, String> = piplock
            .default
            .iter()
            .chain(piplock.develop.iter())
            .map(|dep| {
                (
                    dep.0.to_string(),
                    dep.1["version"].as_str().expect("").to_string(),
                )
            })
            .collect();
        Lockfile { dependencies }
    }
}

impl From<package_json::YarnLock> for Lockfile {
    fn from(yarn_lock: package_json::YarnLock) -> Lockfile {
        let dependencies: HashMap<String, String> = yarn_lock
            .dependencies
            .iter()
            .map(|dep| (dep.0.to_string(), dep.1.to_string()))
            .collect();
        Lockfile { dependencies }
    }
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

pub struct PipfileParser {
    manifest_content: String,
    lockfile_content: String,
}

impl Parser for PipfileParser {
    fn new(manifest_content: &str, lockfile_content: &str) -> Self {
        PipfileParser {
            manifest_content: manifest_content.to_string(),
            lockfile_content: lockfile_content.to_string(),
        }
    }

    fn parse_manifest(&self) -> Result<Manifest, Error> {
        Ok(pipfile::Pipfile::from_str(&self.manifest_content)?.into())
    }

    fn parse_lockfile(&self) -> Result<Lockfile, Error> {
        Ok(pipfile::Piplock::from_str(&self.lockfile_content)?.into())
    }

    fn get_dependencies(&self) -> Result<Vec<DependencyInfo>, Error> {
        let pipfile = self.parse_manifest()?;
        let piplock = self.parse_lockfile()?;

        // Concatenate all dependencie so we can parallelize network calls
        Ok(pipfile
            .dependencies
            .iter()
            .map(|name| {
                let mut line_number: i64 = 0;
                for (index, line) in self.manifest_content.split("\n").enumerate() {
                    if line.to_string().starts_with(&format!("{} = ", name))
                        || line.to_string().starts_with(&format!("\"{}\" = ", name))
                    {
                        line_number = index as i64
                    }
                }
                if let Some(version) = piplock.dependencies.get(name) {
                    let mut v = version.chars();
                    v.next();
                    v.next();
                    DependencyInfo {
                        line_number,
                        name: name.to_string(),
                        current: v.as_str().to_string(),
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

pub struct PackageJsonParser {
    manifest_content: String,
    lockfile_content: String,
}

impl Parser for PackageJsonParser {
    fn new(manifest_content: &str, lockfile_content: &str) -> Self {
        PackageJsonParser {
            manifest_content: manifest_content.to_string(),
            lockfile_content: lockfile_content.to_string(),
        }
    }

    fn parse_manifest(&self) -> Result<Manifest, Error> {
        Ok(package_json::PackageJson::from_str(&self.manifest_content)?.into())
    }

    fn parse_lockfile(&self) -> Result<Lockfile, Error> {
        let lock_file = package_json::YarnLock::from_str(&self.lockfile_content)?;
        Ok(lock_file.into())
    }

    fn get_dependencies(&self) -> Result<Vec<DependencyInfo>, Error> {
        let package_json = self.parse_manifest()?;
        let yarn_lock = self.parse_lockfile()?;

        // Concatenate all dependencie so we can parallelize network calls
        Ok(package_json
            .dependencies
            .iter()
            .map(|name| {
                let mut line_number: i64 = 0;
                for (index, line) in self.manifest_content.split("\n").enumerate() {
                    if line.to_string().contains(&format!("\"{}\": \"", name)) {
                        line_number = index as i64
                    }
                }
                if let Some(version) = yarn_lock.dependencies.get(name) {
                    let mut v = version.chars();
                    v.next();
                    DependencyInfo {
                        line_number,
                        name: name.to_string(),
                        current: v.as_str().to_string(),
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
