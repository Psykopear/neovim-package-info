mod package_json;
mod pipfile;

use crate::consts;
use crate::neovim::DependencyInfo;
use cargo_toml;
use failure::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Manifest {
    dependencies: Vec<(String, String)>,
}

impl From<cargo_toml::Manifest> for Manifest {
    fn from(cargo_toml_manifest: cargo_toml::Manifest) -> Manifest {
        let dependencies: Vec<(String, String)> = cargo_toml_manifest
            .dependencies
            .iter()
            .chain(cargo_toml_manifest.dev_dependencies.iter())
            .chain(cargo_toml_manifest.build_dependencies.iter())
            .map(|(name, requirement)| (name.to_string(), requirement.req().to_string()))
            .collect();
        Manifest { dependencies }
    }
}

impl From<pipfile::Pipfile> for Manifest {
    fn from(pipfile: pipfile::Pipfile) -> Manifest {
        let dependencies: Vec<(String, String)> = pipfile
            .dependencies
            .iter()
            .chain(pipfile.dev_dependencies.iter())
            .map(|(name, requirement)| (name.to_string(), requirement.to_string()))
            .collect();
        Manifest { dependencies }
    }
}

impl From<package_json::PackageJson> for Manifest {
    fn from(package_json: package_json::PackageJson) -> Manifest {
        let dependencies: Vec<(String, String)> = package_json
            .dependencies
            .iter()
            .chain(package_json.dev_dependencies.iter())
            .map(|(name, requirement)| (name.to_string(), requirement.to_string()))
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

#[derive(Serialize, Deserialize)]
pub struct Cargolock {
    pub package: Vec<toml::Value>,
}

impl From<Cargolock> for Lockfile {
    fn from(cargo_lock: Cargolock) -> Lockfile {
        let packages: Vec<toml::Value> = cargo_lock.package.into();
        let dependencies: HashMap<_, _> = packages
            .iter()
            .map(|p| {
                if let Some(name) = p["name"].as_str() {
                    if let Some(version) = p["version"].as_str() {
                        (name.to_string(), version.to_string())
                    } else {
                        (name.to_string(), "0.0.0".to_string())
                    }
                } else {
                    ("error".to_string(), "0.0.0".to_string())
                }
            })
            .collect();
        Lockfile { dependencies }
    }
}

pub trait Parser {
    fn get_dependencies(
        manifest_content: &str,
        lockfile_content: &str,
    ) -> Result<Vec<DependencyInfo>, Error>;
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error>;
    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error>;
}

pub struct CargoParser;

impl Parser for CargoParser {
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error> {
        Ok(cargo_toml::Manifest::from_str(manifest_content)?.into())
    }

    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error> {
        if lockfile_content == "" {
            return Ok(Lockfile {
                dependencies: HashMap::new(),
            });
        }
        let cargo_lock: Cargolock = toml::from_str(lockfile_content)?;
        Ok(cargo_lock.into())
    }

    fn get_dependencies(
        manifest_content: &str,
        lockfile_content: &str,
    ) -> Result<Vec<DependencyInfo>, Error> {
        let cargo_toml = Self::parse_manifest(manifest_content)?;
        let cargo_lock = Self::parse_lockfile(lockfile_content)?;

        // Concatenate all dependencie so we can parallelize network calls
        Ok(cargo_toml
            .dependencies
            .iter()
            .map(|(name, requirement)| {
                let mut line_number: i64 = 0;
                for (index, line) in manifest_content.split("\n").enumerate() {
                    if line.to_string().starts_with(&format!("{} = ", name)) {
                        line_number = index as i64
                    }
                }
                if let Some(version) = cargo_lock.dependencies.get(name) {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: version.to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: "0.0.0".to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                }
            })
            .collect())
    }
}

pub struct PipfileParser;

impl Parser for PipfileParser {
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error> {
        Ok(pipfile::Pipfile::from_str(manifest_content)?.into())
    }

    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error> {
        Ok(pipfile::Piplock::from_str(lockfile_content)?.into())
    }

    fn get_dependencies(
        manifest_content: &str,
        lockfile_content: &str,
    ) -> Result<Vec<DependencyInfo>, Error> {
        let pipfile = Self::parse_manifest(manifest_content)?;
        let piplock = match Self::parse_lockfile(lockfile_content) {
            Ok(lock) => lock,
            Err(_) => Lockfile {
                dependencies: HashMap::new(),
            },
        };

        // Concatenate all dependencie so we can parallelize network calls
        Ok(pipfile
            .dependencies
            .iter()
            .map(|(name, requirement)| {
                let mut line_number: i64 = 0;
                for (index, line) in manifest_content.split("\n").enumerate() {
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
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: v.as_str().to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: "0.0.0".to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                }
            })
            .collect())
    }
}

pub struct PackageJsonParser;

impl Parser for PackageJsonParser {
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error> {
        Ok(package_json::PackageJson::from_str(manifest_content)?.into())
    }

    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error> {
        let lock_file = package_json::YarnLock::from_str(lockfile_content)?;
        Ok(lock_file.into())
    }

    fn get_dependencies(
        manifest_content: &str,
        lockfile_content: &str,
    ) -> Result<Vec<DependencyInfo>, Error> {
        let package_json = Self::parse_manifest(manifest_content)?;
        let yarn_lock = match Self::parse_lockfile(lockfile_content) {
            Ok(lock) => lock,
            Err(_) => Lockfile {
                dependencies: HashMap::new(),
            },
        };

        // Concatenate all dependencie so we can parallelize network calls
        Ok(package_json
            .dependencies
            .iter()
            .map(|(name, requirement)| {
                let mut line_number: i64 = 0;
                for (index, line) in manifest_content.split("\n").enumerate() {
                    if line.to_string().contains(&format!("\"{}\": \"", name)) {
                        line_number = index as i64
                    }
                }
                if let Some(version) = yarn_lock.dependencies.get(name) {
                    let mut v = version.chars();
                    v.next();
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: v.as_str().to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: "0.0.0".to_string(),
                        latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                    }
                }
            })
            .collect())
    }
}
