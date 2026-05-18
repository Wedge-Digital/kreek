use async_trait::async_trait;
use crate::app::competitions::domain::cache_repository_port::{
    CachedSpace, CachedUser, CompetitionsCacheError, ICompetitionsCacheRepository,
};
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CoachId, SpaceId};

pub struct FakeCompetitionsCacheRepository;

#[async_trait]
impl ICompetitionsCacheRepository for FakeCompetitionsCacheRepository {
    async fn add_user(&self, _: &CachedUser)                              -> Result<(), CompetitionsCacheError> { Ok(()) }
    async fn remove_user(&self, _: &CoachId)                              -> Result<(), CompetitionsCacheError> { Ok(()) }
    async fn add_space(&self, _: &CachedSpace)                            -> Result<(), CompetitionsCacheError> { Ok(()) }
    async fn remove_space(&self, _: &SpaceId)                             -> Result<(), CompetitionsCacheError> { Ok(()) }
    async fn subscribe(&self, _: &CoachId, _: &SpaceId, _: &SpaceProfile)         -> Result<(), CompetitionsCacheError> { Ok(()) }
    async fn unsubscribe(&self, _: &CoachId, _: &SpaceId)                 -> Result<(), CompetitionsCacheError> { Ok(()) }
}