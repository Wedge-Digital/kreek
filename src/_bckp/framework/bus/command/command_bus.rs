// use std::fmt;
// use serde::Serialize;
// // use crate::framework::bus::command::command_middleware::CommandMiddleware;
// use crate::framework::bus::message::Message;
// use crate::framework::bus::middleware::{Chainable, MessageHandler};
// use crate::framework::processing::ProcessingResult;
//
// pub struct CommandBus {
//     starting_middleware: Option<CommandMiddleware>
// }
//
// impl fmt::Display for CommandBus {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "CommandBus")
//     }
// }
//
// impl CommandBus {
//     pub fn create() -> Self {
//         return CommandBus{starting_middleware: None};
//     }
//     pub fn new(middleware: &mut Vec<&CommandMiddleware>) -> Self {
//         let mut bus = CommandBus{starting_middleware: None};
//         for mw in middleware.iter_mut() {
//             // bus.starting_middleware.set_next(mw);
//         }
//         return bus;
//     }
// }
//
// // impl<M: Message> MessageHandler<M> for CommandBus {
// //     // fn handle(&mut self, message: &M) -> bool{
// //     //     // return self.starting_middleware.handle(message);
// //     // }
// // }