// use serde::Serialize;
use crate::framework::bus::message::Message;
// use crate::framework::processing::{ProcessingResult};

pub trait Chainable<C: Chainable<C>> {
    fn set_next(&mut self, next: C);

    fn get_next(&self) -> Option<&C>;

    fn has_next(&self) -> bool;
}

pub trait MessageHandler<M: Message> {
    fn handle(&mut self, message: &M) -> bool;
}
