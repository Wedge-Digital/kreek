pub mod path {
    pub const PLAYERS_BY_TEAM_WIDGET: &str = "/app/{space_id}/players/by-team/{team_id}/widget";
    pub const MATCH_PLAYER_SELECTOR: &str = "/app/{space_id}/players/teams/{team_id}/match-selector";
    pub const PLAYER_DEBUG: &str = "/app/{space_id}/players/{player_id}/debug";
    pub const PLAYER_DETAIL: &str = "/app/{space_id}/players/{player_id}/detail";
    pub const PLAYER_SKILLS: &str = "/app/{space_id}/players/{player_id}/skills";
    pub const PLAYER_STAT_INCREASE: &str = "/app/{space_id}/players/{player_id}/stats/{stat}";
    pub const PLAYER_EVOLUTION_JOURNAL_WIDGET: &str =
        "/app/{space_id}/players/{player_id}/widgets/evolution-journal";
    pub const PLAYER_SPP_SPENDING_WIDGET: &str =
        "/app/{space_id}/players/{player_id}/widgets/spp-spending";
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

    pub fn player_debug(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_DEBUG
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn player_detail(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_DETAIL
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn purchase_skill(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_SKILLS
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn increase_stat(&self, space_id: &str, player_id: &str, stat: &str) -> String {
        path::PLAYER_STAT_INCREASE
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
            .replace("{stat}", stat)
    }

    pub fn evolution_journal_widget(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_EVOLUTION_JOURNAL_WIDGET
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn spp_spending_widget(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_SPP_SPENDING_WIDGET
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }
}
