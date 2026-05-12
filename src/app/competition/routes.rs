pub mod path {
    pub const COMPETITION_LIST: &str = "/app/{space_id}/competition";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn all_competitions(&self, sid: &str) -> String { path::COMPETITION_LIST.replace("{space_id}", sid) }
}