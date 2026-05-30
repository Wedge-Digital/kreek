pub mod path {
    pub const DRAFT_TEAM: &str  = "/app/{space_id}/team/create";
    pub const TEAM_BUILD: &str  = "/app/{space_id}/team/{team_id}/build";
    pub const MY_TEAMS:   &str  = "/app/{space_id}/team/list";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn draft_team(&self, space_id: &str) -> String {
        path::DRAFT_TEAM.replace("{space_id}", space_id)
    }
    pub fn team_build(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_BUILD.replace("{space_id}", space_id).replace("{team_id}", team_id)
    }
    pub fn my_teams(&self, space_id: &str) -> String {
        path::MY_TEAMS.replace("{space_id}", space_id)
    }
}