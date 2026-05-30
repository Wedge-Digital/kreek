use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::spaces::domain::space::Space;
use async_trait::async_trait;
use std::fmt;

pub struct SpaceSummary {
    pub id:   String,
    pub name: String,
    pub logo: CloudinaryImage,
}

#[derive(Debug)]
pub enum SpaceRepositoryError {
    SpaceNameAlreadyTaken,
    CoachAlreadyMember,
    Database(String),
}

impl fmt::Display for SpaceRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceRepositoryError::SpaceNameAlreadyTaken => write!(f, "Ce nom d'espace est déjà utilisé"),
            SpaceRepositoryError::CoachAlreadyMember    => write!(f, "Ce coach est déjà membre de cet espace"),
            SpaceRepositoryError::Database(msg)         => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

impl std::error::Error for SpaceRepositoryError {}

#[async_trait]
pub trait ISpaceRepository: Send + Sync {
    async fn save(&self, space: &Space) -> Result<(), SpaceRepositoryError>;
    async fn add_member(&self, space_id: &SpaceId, coach_id: &CoachId, profile: &SpaceProfile) -> Result<(), SpaceRepositoryError>;
    async fn join_spaces(&self, space_ids: &[SpaceId], coach_id: &CoachId) -> Result<(), SpaceRepositoryError>;
    async fn find_by_id(&self, id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError>;
    async fn find_by_coach_id(&self, coach_id: &CoachId) -> Result<Vec<SpaceSummary>, SpaceRepositoryError>;
    async fn find_member_profile(&self, coach_id: &CoachId, space_id: &SpaceId) -> Result<Option<SpaceProfile>, SpaceRepositoryError>;
    async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError>;
}