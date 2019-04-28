mod fetcher;
mod neovim;
mod parser;

fn main() -> Result<(), Box<std::error::Error>> {
    neovim::run();
    Ok(())
}
