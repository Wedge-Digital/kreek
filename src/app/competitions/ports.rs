use async_trait::async_trait;

#[derive(Clone)]
pub struct TeamInfoDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
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

// ── ACL vers le BC `references` (résolution de noms pour l'onglet résumé) ──────

pub trait ICompetitionReferencePort: Send + Sync {
    fn find_inducement_name(&self, uid: &str) -> Option<String>;
    fn find_star_player_name(&self, uid: &str) -> Option<String>;
}

// ── ACL vers le BC `spaces` (profil membre, pour l'autorisation admin) ─────────

#[async_trait]
pub trait ICompetitionSpaceMemberPort: Send + Sync {
    async fn find_member_profile(
        &self,
        coach_id: &crate::app::shared_kernel::common_types::CoachId,
        space_id: &crate::app::shared_kernel::common_types::SpaceId,
    ) -> Option<crate::app::shared_kernel::authorization::SpaceProfile>;
}
