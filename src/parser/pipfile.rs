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
