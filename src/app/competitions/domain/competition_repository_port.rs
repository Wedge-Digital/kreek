use async_trait::async_trait;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;

pub struct CompetitionSummary {
    pub id:     String,
    pub name:   String,
    pub logo:   String,
    pub status: String,
}

#[derive(Debug)]
pub enum CompetitionRepositoryError {
    CompetitionNameAlreadyTaken,
    CompetitionNotFound,
    Database(String),
}

impl std::fmt::Display for CompetitionRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => write!(f, "competition name already taken"),
            CompetitionRepositoryError::CompetitionNotFound         => write!(f, "competition not found"),
            CompetitionRepositoryError::Database(e)                 => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait ICompetitionRepository: Send + Sync {
    async fn name_exists_in_space(&self, name: &CompetitionName, space_id: &SpaceId) -> Result<bool, CompetitionRepositoryError>;
    async fn save(&self, competition: &Competition)                                    -> Result<(), CompetitionRepositoryError>;
    async fn find_by_space_id(&self, space_id: &SpaceId)                              -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError>;
    async fn save_rules(&self, competition_id: &CompetitionId, rules: &CompetitionRules) -> Result<(), CompetitionRepositoryError>;
    async fn find_rules(&self, competition_id: &CompetitionId) -> Result<Option<CompetitionRules>, CompetitionRepositoryError>;
    async fn save_structure(&self, competition_id: &CompetitionId, structure: &CompetitionStructure) -> Result<(), CompetitionRepositoryError>;
    async fn find_structure(&self, competition_id: &CompetitionId) -> Result<Option<CompetitionStructure>, CompetitionRepositoryError>;
    async fn save_invitations(&self, competition_id: &CompetitionId, invitations: &CompetitionInvitations) -> Result<(), CompetitionRepositoryError>;
    async fn find_invitations(&self, competition_id: &CompetitionId) -> Result<Option<CompetitionInvitations>, CompetitionRepositoryError>;
}