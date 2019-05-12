mod cratesio;
mod npm;
mod pypi;

use crate::consts;
use crate::neovim::DependencyInfo;
use failure::Error;
use reqwest;
use semver;
use serde_json;

pub use cratesio::Cratesio;
pub use npm::Npm;
pub use pypi::Pypi;

pub trait Store {
    // A method to retrieve package info given base_url and package name
    // Should be the same for all stores, so we give a default implementation here
    fn get_package_info(package: &str) -> Result<serde_json::Value, Error> {
        let url: String = Self::get_url().replace("{package}", package);
        Ok(reqwest::get(&url)?.json()?)
    }

    // A method to retrieve the last version of a package given its name in the store
    fn get_max_version(package: &str) -> Result<String, Error>;

    /* This should return the full url containing the keyword "{package}"
     * where the package name should be placed in the url (see get_package_info)
     */
    fn get_url() -> String;

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

        if latest_version.major != current.major {
            vec![(
                format!(" -> {}", latest_version),
                consts::RED_HG.to_string(),
            )]
        } else if latest_version.minor != current.minor {
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
        } else if latest_version.patch != current.patch {
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
            vec![]
        }
    }
}
