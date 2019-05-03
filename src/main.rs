mod consts;
mod neovim;
mod parser;
mod store;

fn main() -> Result<(), Box<std::error::Error>> {
    // Just start the event handler and let it go
    neovim::run();
    Ok(())
}
