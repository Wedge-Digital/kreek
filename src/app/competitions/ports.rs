use async_trait::async_trait;

pub struct TeamInfoDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
}

#[async_trait]
pub trait ITeamInfoPort: Send + Sync {
    async fn find_enrolled_teams(&self, season_id: &str) -> Result<Vec<TeamInfoDto>, String>;
}
