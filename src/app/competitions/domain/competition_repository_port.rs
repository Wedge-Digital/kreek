use async_trait::async_trait;
use crate::app::competitions::domain::competition::Competition;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::shared_kernel::competition_name::CompetitionName;

#[derive(Debug)]
pub enum CompetitionRepositoryError {
    CompetitionNameAlreadyTaken,
    Database(String),
}

impl std::fmt::Display for CompetitionRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => write!(f, "competition name already taken"),
            CompetitionRepositoryError::Database(e)                 => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait ICompetitionRepository: Send + Sync {
    async fn name_exists_in_space(&self, name: &CompetitionName, space_id: &SpaceId) -> Result<bool, CompetitionRepositoryError>;
    async fn save(&self, competition: &Competition)                                    -> Result<(), CompetitionRepositoryError>;
}