use crate::app::infra::logger::logger::LogCode::General;
use crate::app::infra::logger::logger::Logger;
use crate::app::infra::logger::memory_logger::MemoryLogger;

#[test]
pub fn test_memory_logger_store_log_string_in_memory() {
    let mut memory_logger = MemoryLogger::new();
    memory_logger.log(General,"Hello from general !");
    assert!(memory_logger.all_logs().len() == 1);
    assert_eq!(memory_logger.all_logs()[0], "GENERAL: Hello from general !");
}