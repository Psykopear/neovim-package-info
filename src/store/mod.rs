use crate::consts;
use failure::Error;
use reqwest;
use semver;
use serde_json;

pub trait Store {
    // A constructor that initializes default values
    fn new() -> Self;

    // A method to retrieve package info given base_url and package name
    // Should be the same for all stores, so we give a default implementation here
    fn get_package_info(&self, package: &str) -> Result<serde_json::Value, Error> {
        let url: String = self.get_url().replace("{package}", package);
        Ok(reqwest::get(&url)?.json()?)
    }

    // A method to retrieve the last version of a package given its name in the store
    fn get_max_version(&self, package: &str) -> Result<String, Error>;

    // Methods to access the structure's fields
    fn get_url(&self) -> &String;
    fn get_name(&self) -> &String;

    // Check dependency and return a string
    fn check_dependency(&self, name: &str, req: &str) -> Vec<(String, String)> {
        if let Ok(store_version) = self.get_max_version(name) {
            if let Ok(latest_version) = semver::Version::parse(&store_version) {
                if let Ok(requirement) = semver::Version::parse(req) {
                    if latest_version.major > requirement.major {
                        vec![(format!("{}", latest_version), consts::RED_HG.to_string())]
                    } else if latest_version.minor > requirement.minor {
                        let split: Vec<String> = latest_version
                            .to_string()
                            .split('.')
                            .map(|x| x.to_string())
                            .collect();
                        vec![
                            (
                                format!("{}.", split[0]).to_string(),
                                consts::GREY_HG.to_string(),
                            ),
                            (split[1..].join("."), consts::BLUE_HG.to_string()),
                        ]
                    } else if latest_version.patch > requirement.patch {
                        let split: Vec<String> = latest_version
                            .to_string()
                            .split('.')
                            .map(|x| x.to_string())
                            .collect();
                        vec![
                            (
                                format!("{}.", split[..2].join(".")).to_string(),
                                consts::GREY_HG.to_string(),
                            ),
                            (split[2..].join("."), "String".to_string()),
                        ]
                    } else {
                        vec![(format!("{}", latest_version), consts::GREY_HG.to_string())]
                    }
                } else {
                    if let Ok(requirement) = semver::VersionReq::parse(req) {
                        if requirement.matches(&latest_version) {
                            vec![(format!("{}", latest_version), consts::GREY_HG.to_string())]
                        } else {
                            vec![(format!("{}", latest_version), "Number".to_string())]
                        }
                    } else {
                        vec![(format!("{}", latest_version), consts::GREY_HG.to_string())]
                    }
                }
            } else {
                vec![(
                    format!("Error parsing store version {}", store_version),
                    consts::GREY_HG.to_string(),
                )]
            }
        } else {
            vec![(
                format!("Error getting store version for {}", name),
                consts::GREY_HG.to_string(),
            )]
        }
    }
}

pub struct Cratesio {
    pub name: String,
    pub base_url: String,
    pub namespace: i64,
}

impl Store for Cratesio {
    fn new() -> Self {
        Self {
            name: "crates.io".to_string(),
            base_url: "https://crates.io/api/v1/crates/{package}".to_string(),
            namespace: 0,
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Error> {
        let body = self.get_package_info(package)?;
        let max_version = body["crate"]["max_version"]
            .as_str()
            .expect("Can't find version");
        Ok(max_version.to_string())
    }
}

pub struct Pypi {
    pub name: String,
    pub base_url: String,
}

impl Store for Pypi {
    fn new() -> Self {
        Self {
            name: "pypi.org".to_string(),
            base_url: "https://pypi.org/pypi/{package}/json".to_string(),
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Error> {
        let body = self.get_package_info(package)?;
        let res = body["info"]["version"]
            .as_str()
            .expect("Can't find version");
        Ok(res.to_string())
    }
}

pub struct Npm {
    pub name: String,
    pub base_url: String,
}

impl Store for Npm {
    fn new() -> Self {
        Self {
            name: "npmjs.org".to_string(),
            base_url: "https://registry.npmjs.org/{package}".to_string(),
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Error> {
        let body = self.get_package_info(package)?;

        let res = body["dist-tags"]["latest"]
            .as_str()
            .expect("Can't find version");
        Ok(res.to_string())
    }
}