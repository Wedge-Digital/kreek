pub mod path {
    pub const DRAFT_TEAM: &str  = "/app/team/create";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn draft_team(&self)    -> &'static str { path::DRAFT_TEAM }
}