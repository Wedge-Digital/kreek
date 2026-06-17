pub mod path {
    pub const PLAYERS_BY_TEAM_WIDGET: &str = "/app/{space_id}/players/by-team/{team_id}/widget";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn players_by_team_widget(&self, space_id: &str, team_id: &str) -> String {
        path::PLAYERS_BY_TEAM_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
}
