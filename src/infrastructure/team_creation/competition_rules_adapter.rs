use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::team_creation::domain::creation_rules::{CreationRules, CreationTier};
use crate::app::team_creation::ports::ICompetitionCreationRulesPort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionRulesAdapter {
    season_repo: Arc<dyn ISeasonRepository>,
}

impl CompetitionRulesAdapter {
    pub fn new(season_repo: Arc<dyn ISeasonRepository>) -> Self {
        Self { season_repo }
    }
}

#[async_trait]
impl ICompetitionCreationRulesPort for CompetitionRulesAdapter {
    async fn find_creation_rules_for_season(&self, season_id: &str) -> Option<CreationRules> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let full = self.season_repo.find_full(&sid).await.ok()??;
        let rules = full.rules?;
        Some(CreationRules {
            tiers: rules
                .tiers
                .into_iter()
                .map(|t| CreationTier {
                    name: t.name,
                    budget: t.budget,
                    start_xp: t.starting_xp,
                    rosters: t.rosters,
                })
                .collect(),
        })
    }
}
