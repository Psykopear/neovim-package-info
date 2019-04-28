mod package_json;

use cargo_toml::Manifest;
use package_json::PackageJson;

pub fn parse_cargo_toml(content: &str) -> Result<Manifest, Box<std::error::Error>> {
    Ok(Manifest::from_str(content)?)
}

pub fn parse_package_json(content: &str) -> Result<PackageJson, Box<std::error::Error>> {
    Ok(PackageJson::from_str(content)?)
}
