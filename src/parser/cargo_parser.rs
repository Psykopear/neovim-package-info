use crate::consts;
use crate::neovim::DependencyInfo;
use crate::parser::{Lockfile, Manifest, Parser};
use failure::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
                        // latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                        latest: vec![(" ".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: "0.0.0".to_string(),
                        // latest: vec![(" ...".to_string(), consts::GREY_HG.to_string())],
                        latest: vec![(" ".to_string(), consts::GREY_HG.to_string())],
                    }
                }
            })
            .collect())
    }
}
