pub mod path {
    pub const COMPETITION_LIST:             &str = "/app/{space_id}/competitions";
    pub const COMPETITION_NEW:              &str = "/app/{space_id}/competitions/create";
    pub const COMPETITION_NEW_MEMBERS:      &str = "/app/{space_id}/competitions/create/members";
    pub const COMPETITION_NEW_RULES:        &str = "/app/{space_id}/competitions/create/{competition_id}/rules";
    pub const COMPETITION_NEW_STRUCTURE:    &str = "/app/{space_id}/competitions/create/{competition_id}/structure";
    pub const COMPETITION_NEW_INVITATIONS: &str = "/app/{space_id}/competitions/create/{competition_id}/invitations";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn all_competitions(&self, sid: &str) -> String { path::COMPETITION_LIST.replace("{space_id}", sid) }
    pub fn new_competition(&self, sid: &str) -> String { path::COMPETITION_NEW.replace("{space_id}", sid) }
    pub fn new_competition_members(&self, sid: &str) -> String { path::COMPETITION_NEW_MEMBERS.replace("{space_id}", sid) }
    pub fn new_competition_rules(&self, sid: &str, cid: &str) -> String {
        path::COMPETITION_NEW_RULES.replace("{space_id}", sid).replace("{competition_id}", cid)
    }
    pub fn new_competition_structure(&self, sid: &str, cid: &str) -> String {
        path::COMPETITION_NEW_STRUCTURE.replace("{space_id}", sid).replace("{competition_id}", cid)
    }
    pub fn new_competition_invitations(&self, sid: &str, cid: &str) -> String {
        path::COMPETITION_NEW_INVITATIONS.replace("{space_id}", sid).replace("{competition_id}", cid)
    }
}