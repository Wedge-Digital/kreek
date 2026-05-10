pub mod path {
    pub const NEW_SPACE:        &str = "/app/space/create";
    pub const NEW_SPACE_SUBMIT: &str = "/app/space/create";
    pub const SPACE_ALL:        &str = "/app/space/all";
    pub const SPACE_JOIN_MANY:  &str = "/app/space/join-many";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn register_space(&self) -> &'static str { path::NEW_SPACE }
    pub fn space_all(&self)      -> &'static str { path::SPACE_ALL }
}