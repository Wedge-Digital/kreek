use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTag {
    pub name:  EventTagName,
    pub value: String,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Serialize)]
pub enum EventTagName {
    User,
    Space,
    Competition,
}

impl ToString for EventTagName {
    fn to_string(&self) -> String {
        match self {
            EventTagName::User => "User".to_string(),
            EventTagName::Space => "Space".to_string(),
            EventTagName::Competition => "Competition".to_string(),
        }
    }
}