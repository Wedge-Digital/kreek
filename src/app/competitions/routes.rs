pub mod path {
    pub const COMPETITION_LIST: &str = "/app/{space_id}/competitions";
    pub const COMPETITION_NEW: &str = "/app/{space_id}/competitions/create";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn all_competitions(&self, sid: &str) -> String { path::COMPETITION_LIST.replace("{space_id}", sid) }
    pub fn new_competition(&self, sid: &str) -> String { path::COMPETITION_NEW.replace("{space_id}", sid) }
}