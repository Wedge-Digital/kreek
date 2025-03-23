// use crate::framework::bus::command::command_bus::CommandBus;
// use crate::framework::bus::command::command_type::Command;
//
// pub fn cmd_test_executor(cmd: &Command) -> &'static str{
//     return "Command executed";
// }
//
// #[test]
// pub fn test_command_bus_is_instanciable() {
//     let commandBus = CommandBus::new();
//     assert_eq!(commandBus.to_string(), "CommandBus");
// }