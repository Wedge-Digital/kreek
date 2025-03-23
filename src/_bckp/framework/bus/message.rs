use std::fmt;
use std::fmt::{Display, Formatter};

pub trait Message {}

pub struct BaseMessage {
    payload: String,
}

impl Message for BaseMessage {}

impl Display for BaseMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "message :: {}", self.payload)
    }
}

impl BaseMessage {
    pub fn new(payload: String) -> BaseMessage {
        BaseMessage { payload }
    }
}
