use async_trait::async_trait;

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;
}
