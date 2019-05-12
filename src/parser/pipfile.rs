use crate::consts;
use crate::neovim::DependencyInfo;
use crate::parser::{Lockfile, Manifest, Parser};
use failure::Error;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Pipfile {
    #[serde(rename = "packages")]
    pub dependencies: HashMap<String, toml::Value>,
    #[serde(rename = "dev-packages")]
    pub dev_dependencies: HashMap<String, toml::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct Piplock {
    pub default: HashMap<String, serde_json::Value>,
    pub develop: HashMap<String, serde_json::Value>,
}

impl Pipfile {
    pub fn from_str(content: &str) -> Result<Self, Error> {
        Ok(toml::from_str(content)?)
    }
}

impl Piplock {
    pub fn from_str(content: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(content)?)
    }
}

impl From<Pipfile> for Manifest {
    fn from(pipfile: Pipfile) -> Manifest {
        let dependencies: Vec<(String, String)> = pipfile
            .dependencies
            .iter()
            .chain(pipfile.dev_dependencies.iter())
            .map(|(name, requirement)| (name.to_string(), requirement.to_string()))
            .collect();
        Manifest { dependencies }
    }
}

impl From<Piplock> for Lockfile {
    fn from(piplock: Piplock) -> Lockfile {
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

pub struct PipfileParser;

impl Parser for PipfileParser {
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error> {
        Ok(Pipfile::from_str(manifest_content)?.into())
    }

    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error> {
        Ok(Piplock::from_str(lockfile_content)?.into())
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
