use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use async_trait::async_trait;

pub struct SeasonBaseInfo {
    pub name: String,
}

pub struct SeasonFull {
    pub season_id: String,
    pub season_name: String,
    pub status: String,
    pub competition_id: String,
    pub competition_name: String,
    pub rules: Option<CompetitionRules>,
    pub structure: Option<CompetitionStructure>,
}

#[derive(Debug)]
pub enum SeasonRepositoryError {
    SeasonNotFound,
    Database(String),
}

impl std::fmt::Display for SeasonRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeasonRepositoryError::SeasonNotFound => write!(f, "season not found"),
            SeasonRepositoryError::Database(e) => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait ISeasonRepository: Send + Sync {
    async fn save(&self, season: &CompetitionSeason) -> Result<(), SeasonRepositoryError>;
    async fn find_latest_season_id(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<SeasonId>, SeasonRepositoryError>;
    async fn find_base_info(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError>;
    async fn find_rules(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionRules>, SeasonRepositoryError>;
    async fn save_rules(
        &self,
        season_id: &SeasonId,
        name: &str,
        rules: &CompetitionRules,
    ) -> Result<(), SeasonRepositoryError>;
    async fn find_structure(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError>;
    async fn save_structure(
        &self,
        season_id: &SeasonId,
        structure: &CompetitionStructure,
    ) -> Result<(), SeasonRepositoryError>;
    async fn find_invitations(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError>;
    async fn save_invitations(
        &self,
        season_id: &SeasonId,
        invitations: &CompetitionInvitations,
    ) -> Result<(), SeasonRepositoryError>;
    async fn set_ready(&self, season_id: &SeasonId) -> Result<(), SeasonRepositoryError>;
    async fn find_full(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonFull>, SeasonRepositoryError>;
}
