use crate::consts;
use crate::neovim::DependencyInfo;
use crate::parser::{Lockfile, Manifest, Parser};
use failure::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct PackageJson {
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: HashMap<String, String>,
}

impl PackageJson {
    pub fn from_str(content: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(content)?)
    }
}

pub struct YarnLock {
    pub dependencies: HashMap<String, String>,
}

impl YarnLock {
    pub fn from_str(content: &str) -> Result<Self, Error> {
        // Because of course the js community couldn't think of anything
        // better than a custom file format to build yarn.lock file, I have to parse it manually.
        // TODO: This implementation seems really fragile, try to improve it
        let lines: Vec<&str> = content.split("\n").collect();
        let mut dependencies: HashMap<String, String> = HashMap::new();
        for (index, line) in lines.iter().enumerate() {
            let line: String = line.to_string();
            if !line.starts_with("#") && !line.starts_with(" ") {
                // Multiple names, we only care about the first one
                if line.contains(",") {
                    if let Some(name) = line.split(",").next() {
                        if let Some(version_separator_index) = name.to_string().rfind("@") {
                            let name = name[..version_separator_index]
                                .to_string()
                                .replace("\"", "");
                            let version: &str = lines[index + 1];
                            if let Some(left) = version.find("\"") {
                                if let Some(right) = version.rfind("\"") {
                                    dependencies.insert(name, version[left..right].to_string());
                                }
                            }
                        }
                    };
                } else {
                    if let Some(name) = line.split(":").next() {
                        if let Some(version_separator_index) = name.to_string().rfind("@") {
                            let name = name[..version_separator_index]
                                .to_string()
                                .replace("\"", "");
                            let version: &str = lines[index + 1];
                            if let Some(left) = version.find("\"") {
                                if let Some(right) = version.rfind("\"") {
                                    dependencies.insert(name, version[left..right].to_string());
                                }
                            }
                        }
                    };
                }
            };
        }

        Ok(YarnLock { dependencies })
    }
}

impl From<PackageJson> for Manifest {
    fn from(package_json: PackageJson) -> Manifest {
        let dependencies: Vec<(String, String)> = package_json
            .dependencies
            .iter()
            .chain(package_json.dev_dependencies.iter())
            .map(|(name, requirement)| (name.to_string(), requirement.to_string()))
            .collect();
        Manifest { dependencies }
    }
}

impl From<YarnLock> for Lockfile {
    fn from(yarn_lock: YarnLock) -> Lockfile {
        let dependencies: HashMap<String, String> = yarn_lock
            .dependencies
            .iter()
            .map(|dep| (dep.0.to_string(), dep.1.to_string()))
            .collect();
        Lockfile { dependencies }
    }
}

pub struct PackageJsonParser;

impl Parser for PackageJsonParser {
    fn parse_manifest(manifest_content: &str) -> Result<Manifest, Error> {
        Ok(PackageJson::from_str(manifest_content)?.into())
    }

    fn parse_lockfile(lockfile_content: &str) -> Result<Lockfile, Error> {
        let lock_file = YarnLock::from_str(lockfile_content)?;
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
                        latest: vec![(" ".to_string(), consts::GREY_HG.to_string())],
                    }
                } else {
                    DependencyInfo {
                        line_number,
                        requirement: requirement.to_string(),
                        name: name.to_string(),
                        current: "0.0.0".to_string(),
                        latest: vec![(" ".to_string(), consts::GREY_HG.to_string())],
                    }
                }
            })
            .collect())
    }
}
