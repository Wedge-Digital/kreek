use async_trait::async_trait;

#[derive(Clone)]
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

    /// Résout des noms d'affichage pour des équipes données, indépendamment de leur
    /// statut d'enrôlement — utilisé pour nommer des équipes exclues d'un appariement
    /// dans un message d'avertissement admin. Les ids introuvables sont omis.
    async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String>;
}
