use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, STATUT_SAISON_PRETE,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::team_creation::domain::creation_rules::{CreationRules, CreationTier};
use crate::app::team_creation::ports::{ICompetitionCreationRulesPort, SeasonCreationData};
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
    /// Traduit la saison telle que `competitions` la connaît vers le
    /// vocabulaire de `team_creation`.
    ///
    /// **Ne tranche rien** : le statut est converti en `prete`, les règles sont
    /// remontées telles quelles, et c'est `season_access_service` qui décide.
    /// L'ancienne version enchaînait trois `?` — identifiant illisible, saison
    /// absente, règles manquantes retombaient sur le même `None`, et le statut
    /// n'était même pas regardé (carte 407).
    async fn find_season_creation_data(&self, season_id: &str) -> Option<SeasonCreationData> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let full = self.season_repo.find_full(&sid).await.ok()??;
        Some(SeasonCreationData {
            prete: full.status == STATUT_SAISON_PRETE,
            statut: full.status,
            rules: full.rules.map(|rules| CreationRules {
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
            }),
        })
    }
}
