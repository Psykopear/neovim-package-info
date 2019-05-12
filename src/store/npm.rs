use crate::store::Store;
use failure::Error;

pub struct Npm;

impl Store for Npm {
    fn get_url() -> String {
        "https://registry.npmjs.org/{package}".to_string()
    }

    fn get_max_version(package: &str) -> Result<String, Error> {
        let body = Self::get_package_info(package)?;

        if let Some(res) = body["dist-tags"]["latest"].as_str() {
            Ok(res.to_string())
        } else {
            Ok("Can't find version".to_string())
        }
    }
}
