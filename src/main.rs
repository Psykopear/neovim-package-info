mod fetcher;
mod neovim;

fn main() -> Result<(), Box<std::error::Error>> {
    neovim::run();
    Ok(())
}
