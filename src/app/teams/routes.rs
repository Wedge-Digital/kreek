pub mod path {
    pub const TEAM_DETAIL: &str = "/app/{space_id}/teams/{team_id}";
    pub const DISMISS_TEAM: &str = "/app/{space_id}/teams/{team_id}/dismiss";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn team_detail(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_DETAIL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn dismiss_team(&self, space_id: &str, team_id: &str) -> String {
        path::DISMISS_TEAM
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
}
