use crate::app::infra::logger::logger::{LogCode, Logger};
use crate::app::infra::middleware::traits::app_message::AppMessage;
use crate::app::infra::middleware::traits::middleware::Middleware;

pub struct LoggerMiddleware<L: Logger>
{
    logger: L,
    next: Option<Box<dyn Middleware>>
}

impl<L:Logger> LoggerMiddleware<L>
{
    pub fn new(logger: &L) -> Self
    {
        return LoggerMiddleware {logger, next: None};
    }
}

impl<L:Logger> Middleware for LoggerMiddleware<L>
{
    fn handle<T: AppMessage>(&self, message: &mut T)
    {
        let printable_message = serde_json::to_string(&message);
        match printable_message {
            Ok(s) => {
                self.logger.log(LogCode::CommandBus, &s);
            },
            Err(_) => {
                self.logger.log(LogCode::CommandBus, "Failed to serialize message");
                return;
            }
        }
        self.next.execute(message);
    }

    fn next(&mut self) -> &mut Option<Box<dyn Middleware>> {
        &mut self.next
    }

    fn set_next<T:Middleware>(&mut self, next: T) {
        self.next = Some(Box::new(next));
    }
}