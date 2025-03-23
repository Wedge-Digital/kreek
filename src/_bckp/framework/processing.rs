use serde::Serialize;

pub type ProcessingResult<T> = Result<ProcessingSuccess<T>, ProcessingError>;

pub struct ProcessingSuccess<T: Serialize> {
    body: T,
    message: String,
    status_code: i32,
    status_label: String,
}

impl<T: Serialize> ProcessingSuccess<T> {
    pub fn new(body: T, message: String, status_code: i32, status_label: String) -> Self {
            return ProcessingSuccess { body, message, status_code, status_label };
        }
}

pub struct ProcessingError {
    status_label: String,
    status_code: i32,
    message: String,
}
