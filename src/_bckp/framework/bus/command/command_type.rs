use crate::framework::bus::message::Message;

#[derive(Debug)]
pub struct Command {
}

impl Message for Command {
}

impl Command {
    pub fn new() -> Self {
        return Command{};
    }
}