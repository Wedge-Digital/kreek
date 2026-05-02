use serde::Serialize;

pub trait DomainEvent: Serialize {
    fn event_type() -> &'static str where Self: Sized;
    fn version()     -> &'static str where Self: Sized;
    fn schema()     -> &'static str where Self: Sized;

    fn serialize_data(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}