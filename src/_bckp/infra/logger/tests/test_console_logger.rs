use crate::app::infra::logger::logger::LogCode::General;
use crate::app::infra::logger::console_logger::ConsoleLogger;
use crate::app::infra::logger::logger::Logger;

#[test]
pub fn test_console_logger() {
    let mut console_logger = ConsoleLogger::new();
    console_logger.log(General,"Hello from general !");
}