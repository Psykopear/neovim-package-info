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
