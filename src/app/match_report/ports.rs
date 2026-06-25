use async_trait::async_trait;

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;
}

pub struct EnrolledTeamDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub team_value: u32,
    pub logo_url: Option<String>,
    pub game_phase: Option<String>,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn list_enrolled_teams(
        &self,
        season_id: &str,
    ) -> Result<Vec<EnrolledTeamDto>, String>;

    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;
}
