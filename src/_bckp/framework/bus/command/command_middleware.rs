// use serde::Serialize;
// use crate::framework::bus::command::command_type::Command;
// use crate::framework::bus::message::Message;
// use crate::framework::bus::middleware::{Chainable, MessageHandler};
// use crate::framework::processing::{ProcessingResult, ProcessingSuccess};

// pub struct CommandMiddleware {
//     is_handled: bool,
//     next: Option<Box<dyn MessageHandler<Command>>>,
// }
//
// impl MessageHandler<Command> for CommandMiddleware {
//     fn handle(&mut self, message: &Command) -> bool {
//         self.is_handled = true;
//         println!("Command message: {:?}", message);
//         let success_body = "Success data".to_string();
//         let success = ProcessingSuccess::new(success_body, "Operation successful".to_string(), 200, "OK".to_string());
//         let processing_result_success: ProcessingResult<String> = Ok(success);
//         return true;
//     }
// }
//
// impl Chainable<CommandMiddleware> for CommandMiddleware {
//     fn set_next(&mut self, next: Box<dyn MessageHandler<Command>>) {
//         // match self.has_next() {
//         //     false => { self.next = Option::from(next) },
//         //     true => { self.get_next().set_next(next) },
//         // }
//     }
//
//     fn get_next(&self) -> Option<&CommandMiddleware> {
//         return Option::from(&self.next);
//     }
//
//     fn has_next(&self) -> bool {
//         return self.next.is_some();
//     }
// }
//

