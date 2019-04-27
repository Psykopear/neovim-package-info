use cargo_toml::Manifest;
use std::fs;

pub fn parse_cargo_toml(content: &str) -> Result<Manifest, Box<std::error::Error>> {
    Ok(Manifest::from_str(content)?)

    //     println!("Dependencies");
    //     for dependency in cargo_toml.dependencies {
    //         println!(
    //             "{}: {:?}",
    //             dependency.0,
    //             VersionReq::parse(dependency.1.req())?
    //         );
    //     }

    // let cargo_toml = content.parse::<Value>()?;
    // // println!("{:?}", cargo_toml);
    // if let build_dependencies = cargo_toml.get("build-dependencies") {
    //     println!("build-dependencies: {:?}", build_dependencies);
    // }

    // println!("build: {:?}", cargo_toml["build-dependencies"]);
    // println!("dev: {:?}", cargo_toml["dev-dependencies"]);
    // println!("deps: {:?}", cargo_toml["dependencies"]);
    // Ok(())
}

pub fn test() -> Result<(), Box<std::error::Error>> {
    let content = fs::read_to_string("./src/parser/examples/Cargo.toml")?;
    parse_cargo_toml(&content)?;
    Ok(())
}
