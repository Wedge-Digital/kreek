use async_trait::async_trait;

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;
}

#[derive(Default)]
pub struct TeamInfoDto {
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;

    async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto>;
}
