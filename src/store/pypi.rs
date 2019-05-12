use crate::store::Store;
use failure::Error;

pub struct Pypi;

impl Store for Pypi {
    fn get_url() -> String {
        "https://pypi.org/pypi/{package}/json".to_string()
    }

    fn get_max_version(package: &str) -> Result<String, Error> {
        let body = Self::get_package_info(package)?;
        if let Some(res) = body["info"]["version"].as_str() {
            Ok(res.to_string())
        } else {
            Ok("Can't find version".to_string())
        }
    }
}
