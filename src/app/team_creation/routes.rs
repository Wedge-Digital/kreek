pub mod path {
    pub const DRAFT_TEAM: &str = "/app/{space_id}/team/create";
    pub const TEAM_BUILD: &str = "/app/{space_id}/team/{team_id}/build";
    pub const MY_TEAMS: &str = "/app/{space_id}/team/list";
    pub const ROSTER_PLAYERS: &str = "/app/{space_id}/team/{team_id}/roster/{roster_uid}/players";
    pub const HIRE_PLAYER: &str = "/app/{space_id}/team/{team_id}/players/hire";
    pub const FIRE_PLAYER: &str = "/app/{space_id}/team/{team_id}/players/fire";
    pub const BUY_STAFF: &str = "/app/{space_id}/team/{team_id}/staff/buy";
    pub const REMOVE_STAFF: &str = "/app/{space_id}/team/{team_id}/staff/remove";
    pub const BUY_REROLL: &str = "/app/{space_id}/team/{team_id}/rerolls/buy";
    pub const REMOVE_REROLL: &str = "/app/{space_id}/team/{team_id}/rerolls/remove";
    pub const SUBMIT_TEAM: &str = "/app/{space_id}/team/{team_id}/submit";
    pub const TEAM_DETAIL: &str = "/app/{space_id}/team/{team_id}/detail";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn draft_team(&self, space_id: &str) -> String {
        path::DRAFT_TEAM.replace("{space_id}", space_id)
    }
    pub fn team_build(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_BUILD
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn my_teams(&self, space_id: &str) -> String {
        path::MY_TEAMS.replace("{space_id}", space_id)
    }
    pub fn roster_players(&self, space_id: &str, team_id: &str, roster_uid: &str) -> String {
        path::ROSTER_PLAYERS
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
            .replace("{roster_uid}", roster_uid)
    }
    pub fn hire_player(&self, space_id: &str, team_id: &str) -> String {
        path::HIRE_PLAYER
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn fire_player(&self, space_id: &str, team_id: &str) -> String {
        path::FIRE_PLAYER
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn buy_staff(&self, space_id: &str, team_id: &str) -> String {
        path::BUY_STAFF
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn remove_staff(&self, space_id: &str, team_id: &str) -> String {
        path::REMOVE_STAFF
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn buy_reroll(&self, space_id: &str, team_id: &str) -> String {
        path::BUY_REROLL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn remove_reroll(&self, space_id: &str, team_id: &str) -> String {
        path::REMOVE_REROLL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn submit_team(&self, space_id: &str, team_id: &str) -> String {
        path::SUBMIT_TEAM
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn team_detail(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_DETAIL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
}
