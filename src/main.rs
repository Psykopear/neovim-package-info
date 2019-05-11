mod cache;
mod consts;
mod neovim;
mod parser;
mod store;

use failure::Error;

fn main() -> Result<(), Error> {
    // Just start the event handler and let it go
    neovim::run();
    Ok(())
}
