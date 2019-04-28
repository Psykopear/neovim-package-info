use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedDep {
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Dep {
    Simple(String),
    Detailed(DetailedDep),
}

#[derive(Serialize, Deserialize)]
pub struct Pipfile {
    #[serde(rename = "packages")]
    pub dependencies: HashMap<String, toml::Value>,
    #[serde(rename = "dev-packages")]
    pub dev_dependencies: HashMap<String, toml::Value>,
}

impl Pipfile {
    pub fn from_str(content: &str) -> Result<Self, Box<std::error::Error>> {
        Ok(toml::from_str(content)?)
    }
}
