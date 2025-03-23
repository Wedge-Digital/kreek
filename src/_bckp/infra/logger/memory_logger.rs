use std::cell::RefCell;
use crate::app::infra::logger::logger::{LogCode, Logger};

pub struct MemoryLogger {
    logs: RefCell<Vec<String>>
}

impl MemoryLogger {
    pub fn new() -> Self {
        return MemoryLogger{logs: RefCell::new(vec![])};
    }
}

impl Logger for MemoryLogger {
    fn log(&self, code: LogCode, message: &str){
        self.logs.borrow_mut().push(format!("{}: {}", code, message));
    }

    fn all_logs(&self) -> Vec<String> {
        return self.logs.borrow().clone();
    }
}