use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::players::ports::{CompetitionAdminInfoDto, IPlayerCompetitionPort};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionAdminAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
}

impl CompetitionAdminAdapter {
    pub fn new(competition_repo: Arc<dyn ICompetitionRepository>) -> Self {
        Self { competition_repo }
    }
}

#[async_trait]
impl IPlayerCompetitionPort for CompetitionAdminAdapter {
    async fn find_admin_info(&self, competition_id: &str) -> Option<CompetitionAdminInfoDto> {
        let id = CompetitionId::try_new(competition_id).ok()?;
        let info = self.competition_repo.find_base_info(&id).await.ok()??;
        Some(CompetitionAdminInfoDto {
            admin_ids: info.admin_ids,
            admin_names: info.admin_names,
        })
    }
}
