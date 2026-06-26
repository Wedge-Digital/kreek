use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::match_report::ports::{ICompetitionDataPort, InducementSpecDto, TierRulesDto};
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::shared_kernel::common_types::SeasonId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionDataAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
    season_repo: Arc<dyn ISeasonRepository>,
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl CompetitionDataAdapter {
    pub fn new(
        competition_repo: Arc<dyn ICompetitionRepository>,
        season_repo: Arc<dyn ISeasonRepository>,
        reference_repo: Arc<dyn IReferenceRepository>,
    ) -> Self {
        Self { competition_repo, season_repo, reference_repo }
    }
}

#[async_trait]
impl ICompetitionDataPort for CompetitionDataAdapter {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String> {
        let comp_id =
            crate::app::shared_kernel::common_types::CompetitionId::try_new(competition_id)
                .map_err(|e| e.to_string())?;
        let info = self
            .competition_repo
            .find_base_info(&comp_id)
            .await
            .map_err(|e| e.to_string())?;

        match info {
            Some(base) => Ok(base.admin_ids.iter().any(|id| id.to_string() == coach_id)),
            None => Ok(false),
        }
    }

    async fn find_tier_rules_for_roster(
        &self,
        season_id: &str,
        roster_id: &str,
    ) -> Option<TierRulesDto> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let rules = self.season_repo.find_rules(&sid).await.ok()??;
        let tier = rules.tiers.iter().find(|t| t.rosters.contains(&roster_id.to_string()))?;
        let allowed_inducements = tier
            .inducements
            .iter()
            .filter_map(|uid| build_inducement_spec(uid, &*self.reference_repo))
            .collect();
        let allowed_star_players = tier
            .star_players
            .iter()
            .filter_map(|uid| build_star_player_spec(uid, &*self.reference_repo))
            .collect();
        Some(TierRulesDto { allowed_inducements, allowed_star_players })
    }
}

fn build_inducement_spec(uid: &str, repo: &dyn IReferenceRepository) -> Option<InducementSpecDto> {
    let ind = repo.find_inducement_by_uid(uid)?;
    Some(InducementSpecDto {
        uid: ind.uid.clone(),
        max_qty: ind.max_quantity as u8,
        unit_cost: ind.cost,
    })
}

fn build_star_player_spec(
    uid: &str,
    repo: &dyn IReferenceRepository,
) -> Option<InducementSpecDto> {
    let sp = repo.find_star_player_by_uid(uid)?;
    Some(InducementSpecDto {
        uid: sp.uid.clone(),
        max_qty: 1,
        unit_cost: sp.cost,
    })
}
