pub mod path {
    pub const MATCH_REPORT_NEW: &str = "/app/{space_id}/match-report/new";
    pub const MATCH_REPORT_EDIT: &str = "/app/{space_id}/match-report/{match_report_id}";
    pub const MATCH_REPORT_TEAMS: &str = "/app/{space_id}/match-report/new/teams";
    pub const MATCH_REPORT_FROM_PAIRING: &str = "/app/{space_id}/match-report/pairing/{pairing_id}";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn new_match_report(&self, space_id: &str) -> String {
        path::MATCH_REPORT_NEW.replace("{space_id}", space_id)
    }

    pub fn edit_match_report(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_EDIT
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn teams_fragment(&self, space_id: &str) -> String {
        path::MATCH_REPORT_TEAMS.replace("{space_id}", space_id)
    }

    pub fn from_pairing(&self, space_id: &str, pairing_id: &str) -> String {
        path::MATCH_REPORT_FROM_PAIRING
            .replace("{space_id}", space_id)
            .replace("{pairing_id}", pairing_id)
    }
}
