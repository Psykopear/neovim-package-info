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

fn main() -> Result<(), Box<std::error::Error>> {
    test()?;
    Ok(())
}
