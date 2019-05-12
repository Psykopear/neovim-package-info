use crate::store::Store;
use failure::Error;

pub struct Cratesio;

impl Store for Cratesio {
    fn get_url() -> String {
        "https://crates.io/api/v1/crates/{package}".to_string()
    }

    fn get_name() -> String {
        "crates.io".to_string()
    }

    fn get_max_version(package: &str) -> Result<String, Error> {
        let body = Self::get_package_info(package)?;
        if let Some(max_version) = body["crate"]["max_version"].as_str() {
            Ok(max_version.to_string())
        } else {
            Ok("Can't find version".to_string())
        }
    }
}
