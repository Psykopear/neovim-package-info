mod neovim;
mod parser;
mod store;

fn main() -> Result<(), Box<std::error::Error>> {
    neovim::run();
    Ok(())
}
