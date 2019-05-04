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
        // better than a custom file format to build yarn.lock file,
        // so I have to parse it manually.
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
