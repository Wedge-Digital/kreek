use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::team_creation::ports::ICompetitionDisplayPort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionDisplayAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
    season_repo: Arc<dyn ISeasonRepository>,
}

impl CompetitionDisplayAdapter {
    pub fn new(competition_repo: Arc<dyn ICompetitionRepository>, season_repo: Arc<dyn ISeasonRepository>) -> Self {
        Self { competition_repo, season_repo }
    }
}

#[async_trait]
impl ICompetitionDisplayPort for CompetitionDisplayAdapter {
    async fn find_competition_name(&self, competition_id: &str) -> Option<String> {
        let id = CompetitionId::try_new(competition_id).ok()?;
        self.competition_repo.find_base_info(&id).await.ok()?.map(|i| i.name)
    }

    async fn find_season_name(&self, season_id: &str) -> Option<String> {
        let id = SeasonId::try_new(season_id).ok()?;
        self.season_repo.find_base_info(&id).await.ok()?.map(|i| i.name)
    }
}
