pub mod path {
    pub const NEW_SPACE:        &str = "/app/space/create";
    pub const NEW_SPACE_SUBMIT: &str = "/app/space/create";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn register_space(&self)    -> &'static str { path::NEW_SPACE }
}