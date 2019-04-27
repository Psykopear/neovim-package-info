use cargo_toml::Dependency;
use cargo_toml::Manifest;
use fetcher::Store;
use semver::{Version, VersionReq};
use std::fs;
use std::path::Path;

mod fetcher;
mod neovim;
mod parser;

fn test() -> Result<(), Box<std::error::Error>> {
    println!("==> Testing fetcher...");
    fetcher::test()?;
    println!("==> Testing parser...");
    parser::test()?;
    println!("==> Running event handler...");
    neovim::run();
    Ok(())
}

fn check_store_dependency<T: Store>(
    package_name: &str,
    dependency: Dependency,
    store: &T,
) -> Result<(), Box<std::error::Error>> {
    let requirement = VersionReq::parse(dependency.req())?;
    let max_version = store.get_max_version(&package_name)?;
    let latest_version = Version::parse(&max_version)?;

    println!("package: {}", package_name);
    println!("requirement: {:?}", requirement);

    if requirement.matches(&latest_version) {
        println!("matches with {:?}", latest_version);
    } else {
        println!("does not match with {:?}", latest_version);
    }

    Ok(())
}

fn check_cargo_toml() -> Result<(), Box<std::error::Error>> {
    let store = fetcher::Cratesio::new();
    let content = fs::read_to_string(
        "/home/docler/study/nvim-plugin/package-info-rs/src/parser/examples/Cargo.toml",
    )?;
    let cargo_toml = parser::parse_cargo_toml(&content)?;
    for dependency in cargo_toml.dependencies {
        check_store_dependency(&dependency.0, dependency.1, &store)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<std::error::Error>> {
    // test()?;
    // check_cargo_toml()?;

    // let content = fs::read_to_string(
    //     "/home/docler/study/nvim-plugin/package-info-rs/src/parser/examples/Cargo.toml",
    // )?;
    // let cargo_toml = Manifest::from_str(&content)?;
    // for (name, _) in cargo_toml.dependencies {
    //     println!("{}", name);
    // }
    neovim::run();
    Ok(())
}
