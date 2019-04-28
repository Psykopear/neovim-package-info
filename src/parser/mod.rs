mod package_json;
mod pipfile;

use cargo_toml::Manifest;

pub fn parse_cargo_toml(content: &str) -> Result<Manifest, Box<std::error::Error>> {
    Ok(Manifest::from_str(content)?)
}

pub fn parse_pipfile(content: &str) -> Result<pipfile::Pipfile, Box<std::error::Error>> {
    Ok(pipfile::Pipfile::from_str(content)?)
}

pub fn parse_package_json(
    content: &str,
) -> Result<package_json::PackageJson, Box<std::error::Error>> {
    Ok(package_json::PackageJson::from_str(content)?)
}
