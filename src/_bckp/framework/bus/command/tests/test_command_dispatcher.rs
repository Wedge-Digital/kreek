// use std::any::Any;
// use crate::framework::bus::command::command_dispatcher::CommandDispatcher;
// use crate::framework::bus::command::command_type::Command;
//
// pub fn cmd_test_executor(cmd: &Command) -> &'static str{
//     return "Command executed";
// }
//
// #[test]
// pub fn assert_register_in_dispatcher_succeed() {
//     let mut dispatcher = CommandDispatcher::new();
//     let command = Command::new();
//     let _command_id = command.type_id();
//     dispatcher.register(_command_id, cmd_test_executor);
//     let result = dispatcher.dispatch(&command);
//     assert_eq!(result, "Command executed");
// }
//
// #[test]
// pub fn assert_register_in_dispatcher_failed() {
//     let mut dispatcher = CommandDispatcher::new();
//     let command = Command::new();
//     let _command_id = command.type_id();
//     let result = dispatcher.dispatch(&command);
//     assert_eq!(result, "No executor candidates found");
// }