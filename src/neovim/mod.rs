use neovim_lib::{Neovim, NeovimApi, Session};

enum Messages {
    Unknown(String),
}

impl From<String> for Messages {
    fn from(event: String) -> Self {
        match &event[..] {
            _ => Messages::Unknown(event),
        }
    }
}

struct EventHandler {
    nvim: Neovim,
}

impl EventHandler {
    fn new() -> Self {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);
        EventHandler { nvim }
    }

    fn recv(&mut self) {
        let receiver = self.nvim.session.start_event_loop_channel();

        for (event, args) in receiver {
            match Messages::from(event) {
                Messages::Unknown(event) => {
                    self.nvim
                        .command(&format!(
                            "echo \"Unkown command: {}, args: {:?}\"",
                            event, args
                        ))
                        .unwrap();
                }
            }
        }
    }
}

pub fn run() {
    let mut event_handler = EventHandler::new();
    event_handler.recv();
}
