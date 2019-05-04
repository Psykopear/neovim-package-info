mod package_json;
mod pipfile;

use cargo_toml::Manifest;
use failure::Error;
use std::collections::HashMap;

pub fn parse_cargo_toml(content: &str) -> Result<Manifest, Error> {
    Ok(Manifest::from_str(content)?)
}

pub fn parse_cargo_lock(content: &str) -> Result<HashMap<String, String>, Error> {
    let cargo_lock: HashMap<String, toml::Value> = toml::from_str(content)?;
    let packages: Vec<toml::Value> = cargo_lock["package"].clone().try_into()?;
    let res: HashMap<_, _> = packages
        .iter()
        .map(|p| {
            (
                p["name"].as_str().expect("").to_string(),
                p["version"].as_str().expect("").to_string(),
            )
        })
        .collect();
    Ok(res)
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
