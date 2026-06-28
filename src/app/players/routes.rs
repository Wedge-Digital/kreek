pub mod path {
    pub const PLAYERS_BY_TEAM_WIDGET: &str = "/app/{space_id}/players/by-team/{team_id}/widget";
    pub const MATCH_PLAYER_SELECTOR: &str = "/app/{space_id}/players/teams/{team_id}/match-selector";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn players_by_team_widget(&self, space_id: &str, team_id: &str) -> String {
        path::PLAYERS_BY_TEAM_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn match_player_selector(&self, space_id: &str, team_id: &str) -> String {
        path::MATCH_PLAYER_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
}
