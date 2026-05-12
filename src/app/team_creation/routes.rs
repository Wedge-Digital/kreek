pub mod path {
    pub const DRAFT_TEAM: &str  = "/app/{space_id}/team/create";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn draft_team(&self, id: &str)    -> String { path::DRAFT_TEAM.replace("{space_id}", id) }
}