use serde_json::{json, Error};
use crate::app::access::login::login_command::LoginCmd;
use crate::app::access::register::register_command::RegisterCmd;
use crate::app::infra::logger::logger::Logger;
use crate::app::infra::middleware::logger_middleware::LoggerMiddleware;
use crate::app::infra::logger::memory_logger::MemoryLogger;
use crate::app::infra::middleware::traits::middleware::Middleware;

#[test]
pub fn log_middleware_should_be_able_to_trace_incomming_request() {
    let memory_logger = MemoryLogger::new();
    let log_middleware = LoggerMiddleware::new(&memory_logger);
    let mut login_cmd = LoginCmd::new("admin".to_string(), "admin".to_string());
    log_middleware.handle(&mut login_cmd);
    assert_eq!(memory_logger.all_logs()[0], "COMMAND_BUS: {\"username\":\"admin\",\"password\":\"admin\"}");
}

#[test]
pub fn log_middleware_should_be_able_to_log_different_commands() {
    let memory_logger = MemoryLogger::new();
    let log_middleware = LoggerMiddleware::new(&memory_logger);
    let mut login_cmd = LoginCmd::new("admin".to_string(), "admin".to_string());

    log_middleware.handle(&mut login_cmd);
    let register_playload = json!({
        "username": "John_Doe",
        "password": "password",
        "email": "toto@gmail.com",
        "firstname": "John",
        "lastname": "Doe",
        "coach_name": "Doe"
    });
    let mut register_cmd:Result<RegisterCmd, Error> = serde_json::from_value(register_playload);
    match register_cmd {
        Ok(mut cmd) => {
            let expected_register_cmd_str = serde_json::to_string(&cmd).expect("Failed to serialize register command");
            log_middleware.handle(&mut cmd);
            assert_eq!(memory_logger.all_logs().len(), 2);
            assert_eq!(memory_logger.all_logs()[0], "COMMAND_BUS: {\"username\":\"admin\",\"password\":\"admin\"}");
            assert_eq!(memory_logger.all_logs()[1], format!("COMMAND_BUS: {}", expected_register_cmd_str));
        },
        Err(_) => {
            panic!("Failed to serialize register command");
        }
    }
}

#[test]
pub fn test_log_middleware_should_be_chainable() {
    let memory_logger = MemoryLogger::new();
    let mut first_log_middleware = LoggerMiddleware::new(&memory_logger);
    let mut second_log_middleware = LoggerMiddleware::new(&memory_logger);

    first_log_middleware.set_next(&second_log_middleware);

    let mut login_cmd = LoginCmd::new("admin".to_string(), "admin".to_string());
    first_log_middleware.handle(&mut login_cmd);

    assert_eq!(memory_logger.all_logs().len(), 2);
    assert_eq!(memory_logger.all_logs()[0], "COMMAND_BUS: {\"username\":\"admin\",\"password\":\"admin\"}");
    assert_eq!(memory_logger.all_logs()[1], "COMMAND_BUS: {\"username\":\"admin\",\"password\":\"admin\"}");
}