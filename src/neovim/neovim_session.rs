use crate::consts;
use neovim_lib::neovim_api::Buffer;
use neovim_lib::{Neovim, NeovimApi, Session, Value};

pub struct NeovimSession {
    pub nvim: Neovim,
    pub buffer_number: i64,
}

impl NeovimSession {
    pub fn new() -> Self {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);
        NeovimSession {
            nvim,
            buffer_number: 0,
        }
    }

    pub fn echo(&mut self, message: &str) {
        self.nvim.command(&format!("echo \"{}\"", message)).unwrap();
    }

    fn get_buffer(&mut self) -> Option<Buffer> {
        let buffers = self.nvim.list_bufs().expect("Error listing buffers");
        for buf in buffers {
            if buf
                .get_number(&mut self.nvim)
                .expect("Error getting buffer number")
                == self.buffer_number
            {
                return Some(buf);
            }
        }
        None
    }

    pub fn set_text(&mut self, messages: &Vec<(String, String)>, line_number: i64) {
        if let Some(buffer) = self.get_buffer() {
            let mut chunks: Vec<Value> = messages
                .iter()
                .map(|(message, highlight)| {
                    vec![
                        Value::from(message.to_string()),
                        Value::from(highlight.to_string()),
                    ]
                    .into()
                })
                .collect();
            chunks.insert(
                0,
                vec![Value::from(consts::PREFIX), Value::from(consts::GREY_HG)].into(),
            );
            match buffer.set_virtual_text(&mut self.nvim, 0, line_number, chunks, vec![]) {
                Ok(_) => (),
                Err(error) => self.echo(&format!("{}", error)),
            }
        }
    }

    pub fn start_event_loop_channel(&mut self) -> std::sync::mpsc::Receiver<(String, Vec<Value>)> {
        self.nvim.session.start_event_loop_channel()
    }
}
