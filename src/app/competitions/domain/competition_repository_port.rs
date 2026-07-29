use crate::app::competitions::domain::competition::Competition;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use async_trait::async_trait;

pub struct SeasonOption {
    pub season_id: String,
    pub season_name: String,
    pub status: String,
}

pub struct CompetitionWithSeasons {
    pub competition_id: String,
    pub competition_name: String,
    pub seasons: Vec<SeasonOption>,
}

pub struct CompetitionSummary {
    pub id: String,
    pub name: String,
    pub logo: String,
    pub season_id: Option<String>,
    pub status: Option<String>,
    pub season_count: i64,
}

pub struct CompetitionBaseInfo {
    pub name: String,
    pub logo: Option<String>,
    pub admin_ids: Vec<String>,
    pub admin_names: Vec<String>,
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
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => {
                write!(f, "competition name already taken")
            }
            CompetitionRepositoryError::CompetitionNotFound => write!(f, "competition not found"),
            CompetitionRepositoryError::Database(e) => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait ICompetitionRepository: Send + Sync {
    async fn name_exists_in_space(
        &self,
        name: &CompetitionName,
        space_id: &SpaceId,
    ) -> Result<bool, CompetitionRepositoryError>;
    async fn save(&self, competition: &Competition) -> Result<(), CompetitionRepositoryError>;
    async fn find_by_space_id(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError>;
    async fn find_with_seasons(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<CompetitionWithSeasons>, CompetitionRepositoryError>;
    async fn find_base_info(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError>;
    async fn update_base_info(
        &self,
        competition_id: &CompetitionId,
        name: &CompetitionName,
        logo: &CloudinaryImage,
        admin_ids: &[CoachId],
    ) -> Result<(), CompetitionRepositoryError>;
}
