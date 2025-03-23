use serde::Serialize;

pub trait Handler {
    fn handle<S:Serialize>(&self, message: &S);
}