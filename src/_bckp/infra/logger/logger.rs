use std::fmt;

pub enum LogCode {
    General = 0,
    CommandBus = 1,
}

impl fmt::Display for LogCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogCode::General => write!(f, "GENERAL"),
            LogCode::CommandBus => write!(f, "COMMAND_BUS"),
        }
    }
}

pub trait Logger {
    fn log(&self, code: LogCode, message: &str);

    fn all_logs(&self) -> Vec<String>;
}