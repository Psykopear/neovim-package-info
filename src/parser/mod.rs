mod package_json;
mod pipfile;

use cargo_toml::Manifest;
use failure::Error;

pub fn parse_cargo_toml(content: &str) -> Result<Manifest, Error> {
    Ok(Manifest::from_str(content)?)
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
