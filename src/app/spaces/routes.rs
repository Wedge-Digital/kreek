use serde::{Deserialize, Serialize};

pub mod path {
    pub const NEW_SPACE:        &str = "/app/space/create";
    pub const SPACE_ALL:        &str = "/app/space/all";
    pub const SPACE_JOIN:  &str = "/app/space/join";
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Routes;

impl Routes {
    pub fn register_space(&self) -> &'static str { path::NEW_SPACE }
    pub fn space_all(&self)      -> &'static str { path::SPACE_ALL }

    pub fn join(&self)           -> &'static str { path::SPACE_JOIN }
}