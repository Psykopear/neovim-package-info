use reqwest;
use serde_json;

pub trait Store {
    // A constructor that initializes default values
    fn new() -> Self;
    // A method to retrieve package info given base_url and package name
    // Should be the same for all stores, so we give a default implementation here
    fn get_package_info(&self, package: &str) -> Result<serde_json::Value, Box<std::error::Error>> {
        let url: String = self.get_url().replace("{package}", package);
        Ok(reqwest::get(&url)?.json()?)
    }
    // A method to retrieve the last version of a package given its name in the store
    fn get_max_version(&self, package: &str) -> Result<String, Box<std::error::Error>>;
    // Methods to access the structure's fields
    fn get_url(&self) -> &String;
    fn get_name(&self) -> &String;
}

// Crates.io
struct Cratesio {
    pub name: String,
    pub base_url: String,
}

impl Store for Cratesio {
    fn new() -> Self {
        Self {
            name: "crates io".to_string(),
            base_url: "https://crates.io/api/v1/crates/{package}".to_string(),
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Box<std::error::Error>> {
        let body = self.get_package_info(package)?;
        Ok(body["crate"]["max_version"].to_string())
    }
}

// Pypi
struct Pypi {
    pub name: String,
    pub base_url: String,
}

impl Store for Pypi {
    fn new() -> Self {
        Self {
            name: "pypi".to_string(),
            base_url: "https://pypi.org/pypi/{package}/json".to_string(),
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Box<std::error::Error>> {
        let body = self.get_package_info(package)?;
        Ok(body["info"]["version"].to_string())
    }
}

// NPM
struct Npm {
    pub name: String,
    pub base_url: String,
}

impl Store for Npm {
    fn new() -> Self {
        Self {
            name: "npm".to_string(),
            base_url: "https://registry.npmjs.org/{package}".to_string(),
        }
    }

    fn get_url(&self) -> &String {
        &self.base_url
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_max_version(&self, package: &str) -> Result<String, Box<std::error::Error>> {
        let body = self.get_package_info(package)?;
        Ok(body["dist-tags"]["latest"].to_string())
    }
}

fn test_single_store<T: Store>(store: T, package: &str) -> Result<(), Box<std::error::Error>> {
    let max_version = store.get_max_version(&package)?;
    let name = store.get_name();
    println!("Max {} version on {}: {}", package, name, max_version);
    Ok(())
}

fn test_stores() -> Result<(), Box<std::error::Error>> {
    test_single_store(Cratesio::new(), "reqwest")?;
    test_single_store(Pypi::new(), "requests")?;
    test_single_store(Npm::new(), "axios")?;
    Ok(())
}

pub fn test() -> Result<(), Box<std::error::Error>> {
    test_stores()?;
    Ok(())
}
