use crate::framework::bus::message::BaseMessage;

pub trait Middleware {
    type MiddlewareType;
    fn handle(&mut self, message: BaseMessage) -> BaseMessage;
    fn set_next(&mut self, next: Self::MiddlewareType);
    fn get_next(&self) -> Option<&Self::MiddlewareType>;
}
