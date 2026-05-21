use async_trait::async_trait;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::shared_kernel::common_types::SpaceId;
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
}