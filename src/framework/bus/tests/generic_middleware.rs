use crate::framework::bus::middleware::middleware::Middleware;
use crate::framework::bus::message::BaseMessage;

pub struct GenericMessageMiddleware {
    next: Option<Box<GenericMessageMiddleware>>,
    is_handled: bool,
}

impl GenericMessageMiddleware {
    pub fn new() -> Self {
        return GenericMessageMiddleware { next: None, is_handled: false };
    }

    pub fn is_handled(&self) -> bool {
        self.is_handled
    }
}

impl Middleware for GenericMessageMiddleware {

    type MiddlewareType = GenericMessageMiddleware;

    fn handle(&mut self, message: BaseMessage) -> BaseMessage {
        self.is_handled = true;
        match &mut self.next {
            Some(next) => next.handle(message),
            None => message,
        }
    }

    fn set_next(&mut self, next: Self::MiddlewareType) {
        self.next = Some(Box::new(next));
    }

    fn get_next(&self) -> Option<&Self::MiddlewareType> {
        match &self.next {
            Some(next) => Some(next.as_ref()),
            None => None,
        }
    }
}