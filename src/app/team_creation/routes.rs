pub mod path {
    pub const DRAFT_TEAM:          &str = "/app/{space_id}/team/create";
    pub const TEAM_BUILD:          &str = "/app/{space_id}/team/{team_id}/build";
    pub const MY_TEAMS:            &str = "/app/{space_id}/team/list";
    pub const ROSTER_PLAYERS:      &str = "/app/{space_id}/team/{team_id}/roster/{roster_uid}/players";
    pub const HIRE_PLAYER:         &str = "/app/{space_id}/team/{team_id}/players/hire";
    pub const FIRE_PLAYER:         &str = "/app/{space_id}/team/{team_id}/players/fire";
    pub const BUY_STAFF:           &str = "/app/{space_id}/team/{team_id}/staff/buy";
    pub const REMOVE_STAFF:        &str = "/app/{space_id}/team/{team_id}/staff/remove";
    pub const BUY_REROLL:          &str = "/app/{space_id}/team/{team_id}/rerolls/buy";
    pub const REMOVE_REROLL:       &str = "/app/{space_id}/team/{team_id}/rerolls/remove";
    pub const SUBMIT_TEAM:         &str = "/app/{space_id}/team/{team_id}/submit";
    pub const TEAM_DETAIL:         &str = "/app/{space_id}/team/{team_id}/detail";
    pub const SET_PLAYER_IDENTITY: &str = "/app/{space_id}/team/{team_id}/players/{instance_id}/identity";
    pub const SET_LEAGUE:          &str = "/app/{space_id}/team/{team_id}/league";
    pub const SET_SPECIAL_RULE:    &str = "/app/{space_id}/team/{team_id}/special-rule";
    pub const FINALIZE_TEAM:       &str = "/app/{space_id}/team/{team_id}/finalize";
    pub const CART_WIDGET:         &str = "/app/{space_id}/team/{team_id}/widgets/cart";
    pub const ROSTER_PICKER_WIDGET: &str = "/app/{space_id}/team/{team_id}/widgets/roster-picker";
    pub const PLAYER_TABLE_WIDGET: &str = "/app/{space_id}/team/{team_id}/widgets/player-table";
    pub const STAFF_TABLE_WIDGET:  &str = "/app/{space_id}/team/{team_id}/widgets/staff-table";
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
    pub fn set_player_identity(&self, space_id: &str, team_id: &str, instance_id: &str) -> String {
        path::SET_PLAYER_IDENTITY
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
            .replace("{instance_id}", instance_id)
    }
    pub fn set_league(&self, space_id: &str, team_id: &str) -> String {
        path::SET_LEAGUE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn set_special_rule(&self, space_id: &str, team_id: &str) -> String {
        path::SET_SPECIAL_RULE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn finalize_team(&self, space_id: &str, team_id: &str) -> String {
        path::FINALIZE_TEAM
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn staff_table_widget(&self, space_id: &str, team_id: &str) -> String {
        path::STAFF_TABLE_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn player_table_widget(&self, space_id: &str, team_id: &str) -> String {
        path::PLAYER_TABLE_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn roster_picker_widget(&self, space_id: &str, team_id: &str) -> String {
        path::ROSTER_PICKER_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn cart_widget(&self, space_id: &str, team_id: &str) -> String {
        path::CART_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
}
