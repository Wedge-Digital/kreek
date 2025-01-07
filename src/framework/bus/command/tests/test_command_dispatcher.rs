use std::any::Any;
use crate::framework::bus::command::command_dispatcher::CommandDispatcher;
use crate::framework::bus::command::command_type::Command;

pub fn cmd_test_executor(cmd: &Command) -> CommandResult {
    println!("Executing command: {:?}", cmd);
}

#[test]
pub fn assert_register_in_dispatcher_succeed() {
    let mut dispatcher = CommandDispatcher::new();
    let command = Command::new();
    let _command_id = command.type_id();
    dispatcher.register(_command_id, cmd_test_executor);
    let result = dispatcher.dispatch(&command);
}