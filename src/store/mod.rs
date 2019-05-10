use crate::consts;
use crate::neovim::DependencyInfo;
use failure::Error;
use reqwest;
use semver;
use serde_json;

pub trait Store {
    // A method to retrieve package info given base_url and package name
    // Should be the same for all stores, so we give a default implementation here
    fn get_package_info(package: &str) -> Result<serde_json::Value, Error> {
        let url: String = Self::get_url().replace("{package}", package);
        Ok(reqwest::get(&url)?.json()?)
    }

    // A method to retrieve the last version of a package given its name in the store
    fn get_max_version(package: &str) -> Result<String, Error>;

    // Methods to access the structure's fields
    fn get_url() -> String;
    fn get_name() -> String;

    // Check dependency and return a string
    fn check_dependency(dep: &DependencyInfo) -> Vec<(String, String)> {
        // Get store version first
        let store_version = match Self::get_max_version(&dep.name) {
            Ok(store_version) => store_version,
            Err(_) => {
                return vec![(
                    format!(" -> Error retrieving version for {}", dep.name),
                    consts::GREY_HG.to_string(),
                )]
            }
        };

        // Current from lockfile
        let current = match semver::Version::parse(&dep.current) {
            Ok(current) => current,
            Err(_) => return vec![(format!(" {}", store_version), consts::GREY_HG.to_string())],
        };

        // Latest store version
        let latest_version = match semver::Version::parse(&store_version) {
            Ok(latest_version) => latest_version,
            Err(_) => return vec![(format!(" {}", store_version), consts::GREY_HG.to_string())],
        };

        if latest_version.major > current.major {
            vec![(
                format!(" -> {}", latest_version),
                consts::RED_HG.to_string(),
            )]
        } else if latest_version.minor > current.minor {
            let split: Vec<String> = latest_version
                .to_string()
                .split('.')
                .map(|x| x.to_string())
                .collect();
            vec![
                (
                    format!(" -> {}.", split[0]).to_string(),
                    consts::GREY_HG.to_string(),
                ),
                (split[1..].join("."), consts::BLUE_HG.to_string()),
            ]
        } else if latest_version.patch > current.patch {
            let split: Vec<String> = latest_version
                .to_string()
                .split('.')
                .map(|x| x.to_string())
                .collect();
            vec![
                (
                    format!(" -> {}.", split[..2].join(".")).to_string(),
                    consts::GREY_HG.to_string(),
                ),
                (split[2..].join("."), "String".to_string()),
            ]
        } else {
            // vec![(format!("{}", latest_version), consts::GREY_HG.to_string())]
            vec![]
        }
    }
}

////////////////////////////////////
// crates.io Store implementation //
////////////////////////////////////
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

///////////////////////////////
// Pypi Store implementation //
///////////////////////////////
pub struct Pypi;

impl Store for Pypi {
    fn get_url() -> String {
        "https://pypi.org/pypi/{package}/json".to_string()
    }

    fn get_name() -> String {
        "pypi.org".to_string()
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

//////////////////////////////
// NPM Store implementation //
//////////////////////////////
pub struct Npm;

impl Store for Npm {
    fn get_url() -> String {
        "https://registry.npmjs.org/{package}".to_string()
    }

    fn get_name() -> String {
        "npmjs.org".to_string()
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
