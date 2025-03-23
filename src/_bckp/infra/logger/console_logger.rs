use std::cell::RefCell;
use crate::app::infra::logger::logger::{LogCode, Logger};

pub struct ConsoleLogger {
    logs: RefCell<Vec<String>>
}

impl ConsoleLogger {
    pub fn new() -> Self {
        return ConsoleLogger{logs: RefCell::new(vec![])};
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, code: LogCode, message: &str){
        println!("{}: {}", code, message);
    }

    fn all_logs(&self) -> Vec<String> {
        return self.logs.borrow().clone();
    }
}