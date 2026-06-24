use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::match_report::ports::{
    CompetitionOptionDto, ICompetitionDataPort, RoundOptionDto, SeasonOptionDto,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionDataAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
    match_day_repo: Arc<dyn IMatchDayRepository>,
}

impl CompetitionDataAdapter {
    pub fn new(
        competition_repo: Arc<dyn ICompetitionRepository>,
        match_day_repo: Arc<dyn IMatchDayRepository>,
    ) -> Self {
        Self {
            competition_repo,
            match_day_repo,
        }
    }
}

#[async_trait]
impl ICompetitionDataPort for CompetitionDataAdapter {
    async fn list_competitions_with_active_season(
        &self,
        space_id: &str,
    ) -> Result<Vec<CompetitionOptionDto>, String> {
        let space_id =
            crate::app::shared_kernel::common_types::SpaceId::try_new(space_id)
                .map_err(|e| e.to_string())?;
        let comps = self
            .competition_repo
            .find_with_seasons(&space_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(comps
            .into_iter()
            .filter(|c| !c.seasons.is_empty())
            .map(|c| CompetitionOptionDto {
                competition_id: c.competition_id,
                name: c.competition_name,
            })
            .collect())
    }

    async fn list_seasons(
        &self,
        competition_id: &str,
    ) -> Result<Vec<SeasonOptionDto>, String> {
        let space_id = crate::app::shared_kernel::common_types::SpaceId::try_new(
            "00000000000000000000000000",
        )
        .map_err(|e| e.to_string())?;

        let comps = self
            .competition_repo
            .find_with_seasons(&space_id)
            .await
            .map_err(|e| e.to_string())?;

        let comp = comps.into_iter().find(|c| c.competition_id == competition_id);
        match comp {
            Some(c) => Ok(c
                .seasons
                .into_iter()
                .rev()
                .map(|s| SeasonOptionDto {
                    season_id: s.season_id,
                    name: s.season_name,
                })
                .collect()),
            None => Ok(vec![]),
        }
    }

    async fn list_rounds(&self, season_id: &str) -> Result<Vec<RoundOptionDto>, String> {
        let match_days = self
            .match_day_repo
            .find_by_season(season_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(match_days
            .into_iter()
            .filter(|md| !md.is_rest())
            .map(|md| RoundOptionDto {
                round_id: md.id,
                name: md.name,
                date_start: md.date_start,
                date_end: md.date_end,
            })
            .collect())
    }

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
}
