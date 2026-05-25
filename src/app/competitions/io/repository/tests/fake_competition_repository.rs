use async_trait::async_trait;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_repository_port::{CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;

pub struct FakeCompetitionRepository;

#[async_trait]
impl ICompetitionRepository for FakeCompetitionRepository {
    async fn name_exists_in_space(&self, _: &CompetitionName, _: &SpaceId) -> Result<bool, CompetitionRepositoryError> {
        Ok(false)
    }
    async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn find_by_space_id(&self, _: &SpaceId) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
        Ok(vec![])
    }
    async fn save_rules(&self, _: &CompetitionId, _: &CompetitionRules) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn find_rules(&self, _: &CompetitionId) -> Result<Option<CompetitionRules>, CompetitionRepositoryError> {
        Ok(None)
    }
    async fn save_structure(&self, _: &CompetitionId, _: &CompetitionStructure) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn find_structure(&self, _: &CompetitionId) -> Result<Option<CompetitionStructure>, CompetitionRepositoryError> {
        Ok(None)
    }
    async fn save_invitations(&self, _: &CompetitionId, _: &CompetitionInvitations) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn find_invitations(&self, _: &CompetitionId) -> Result<Option<CompetitionInvitations>, CompetitionRepositoryError> {
        Ok(None)
    }
    async fn find_base_info(&self, _: &CompetitionId) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
        Ok(None)
    }
    async fn update_base_info(&self, _: &CompetitionId, _: &CompetitionName, _: &CloudinaryImage, _: &[CoachId]) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
    async fn set_ready(&self, _: &CompetitionId) -> Result<(), CompetitionRepositoryError> {
        Ok(())
    }
}