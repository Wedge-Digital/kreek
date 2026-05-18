use async_trait::async_trait;
use serde::Deserialize;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::space_name::SpaceName;

#[derive(Debug)]
pub enum CompetitionsCacheError {
    Database(String),
}

impl std::fmt::Display for CompetitionsCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompetitionsCacheError::Database(e) => write!(f, "CompetitionsCacheError: {}", e),
        }
    }
}

#[derive(Clone)]
pub struct CachedUser {
    pub id:         CoachId,
    pub coach_name: CoachName,
    pub coach_icon: Option<CoachIcon>,
    pub email:      Email,
}


#[derive(Deserialize, Debug, Clone)]
pub struct CachedSpace {
    pub space_id:        SpaceId,
    pub space_name:      SpaceName,
    pub space_logo:      CloudinaryImage,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserSubscription {
    pub coach_id:      CoachId,
    pub space_id:      SpaceId,
    pub user_profile:  SpaceProfile,
}



#[async_trait]
pub trait ICompetitionsCacheRepository: Send + Sync {
    async fn add_user(&self, user: &CachedUser)              -> Result<(), CompetitionsCacheError>;
    async fn remove_user(&self, id: &CoachId)                -> Result<(), CompetitionsCacheError>;
    async fn add_space(&self, space: &CachedSpace)           -> Result<(), CompetitionsCacheError>;
    async fn remove_space(&self, space_id: &SpaceId)         -> Result<(), CompetitionsCacheError>;
    async fn subscribe(&self, coach_id: &CoachId, space_id: &SpaceId, profile: &SpaceProfile) -> Result<(), CompetitionsCacheError>;
    async fn unsubscribe(&self, coach_id: &CoachId, space_id: &SpaceId)              -> Result<(), CompetitionsCacheError>;
}