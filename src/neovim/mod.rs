mod cache;
mod event_handler;
mod neovim_session;

use event_handler::EventHandler;
use neovim_session::NeovimSession;

pub use event_handler::DependencyInfo;

pub fn run() {
    let mut nvim_session = NeovimSession::new();
    EventHandler::recv(&mut nvim_session);
}
