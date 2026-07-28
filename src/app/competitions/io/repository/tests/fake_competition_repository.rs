use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{
    CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, CompetitionWithSeasons,
    ICompetitionRepository,
};
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use async_trait::async_trait;

pub struct FakeCompetitionRepository;

#[async_trait]
impl ICompetitionRepository for FakeCompetitionRepository {
    async fn name_exists_in_space(
        &self,
        _: &CompetitionName,
        _: &SpaceId,
    ) -> Result<bool, CompetitionRepositoryError> {
        Ok(false)
    }
    async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn find_by_space_id(
        &self,
        _: &SpaceId,
    ) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
        Ok(vec![])
    }
    async fn find_base_info(
        &self,
        _: &CompetitionId,
    ) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
        Ok(None)
    }
    async fn find_with_seasons(
        &self,
        _: &SpaceId,
    ) -> Result<Vec<CompetitionWithSeasons>, CompetitionRepositoryError> {
        Ok(vec![])
    }
    async fn update_base_info(
        &self,
        _: &CompetitionId,
        _: &CompetitionName,
        _: &CloudinaryImage,
        _: &[CoachId],
    ) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
}
