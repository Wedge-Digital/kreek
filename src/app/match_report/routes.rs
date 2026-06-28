pub mod path {
    pub const MATCH_REPORT_NEW: &str = "/app/{space_id}/match-report/new";
    pub const MATCH_REPORT_EDIT: &str = "/app/{space_id}/match-report/{match_report_id}";
    pub const MATCH_REPORT_FROM_PAIRING: &str = "/app/{space_id}/match-report/pairing/{pairing_id}";
    pub const MATCH_REPORT_STEP2: &str = "/app/{space_id}/match-report/{match_report_id}/step2";
    pub const MATCH_REPORT_INDUCEMENTS: &str =
        "/app/{space_id}/match-report/{match_report_id}/inducements/{team_id}";
    pub const MATCH_REPORT_STEP3: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3";
    pub const MATCH_REPORT_STEP4: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4";
    pub const MATCH_REPORT_STEP3_TURN_SELECTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/turn-selector";
    pub const MATCH_REPORT_STEP4_TURN_SELECTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/turn-selector";
    pub const MATCH_REPORT_STEP3_TEMP_PLAYERS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/temp-players";
    pub const MATCH_REPORT_STEP4_TEMP_PLAYERS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/temp-players";
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

    pub fn from_pairing(&self, space_id: &str, pairing_id: &str) -> String {
        path::MATCH_REPORT_FROM_PAIRING
            .replace("{space_id}", space_id)
            .replace("{pairing_id}", pairing_id)
    }

    pub fn step2(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP2
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn inducements(&self, space_id: &str, match_report_id: &str, team_id: &str) -> String {
        path::MATCH_REPORT_INDUCEMENTS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{team_id}", team_id)
    }

    pub fn step3(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3_turn_selector(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_TURN_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_turn_selector(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_TURN_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3_temp_players(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_TEMP_PLAYERS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_temp_players(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_TEMP_PLAYERS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }
}
