use async_trait::async_trait;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
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
}