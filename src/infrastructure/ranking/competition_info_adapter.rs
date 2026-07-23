use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::ranking::ports::{
    EnrolledTeamInfo, IRankingCompetitionPort, RankingGroupInfo, RankingRulesInfo,
};
use crate::app::shared_kernel::common_types::SeasonId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RankingCompetitionAdapter {
    season_repo: Arc<dyn ISeasonRepository>,
    team_info_port: Arc<dyn ITeamInfoPort>,
    group_repo: Arc<dyn IGroupRepository>,
}

impl RankingCompetitionAdapter {
    pub fn new(
        season_repo: Arc<dyn ISeasonRepository>,
        team_info_port: Arc<dyn ITeamInfoPort>,
        group_repo: Arc<dyn IGroupRepository>,
    ) -> Self {
        Self { season_repo, team_info_port, group_repo }
    }
}

#[async_trait]
impl IRankingCompetitionPort for RankingCompetitionAdapter {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo> {
        let sid = SeasonId::try_new(season_id).ok()?;
        let rules = self.season_repo.find_rules(&sid).await.ok()??;
        Some(RankingRulesInfo {
            win_points: rules.ranking_rules.win_points.into_inner(),
            draw_points: rules.ranking_rules.draw_points.into_inner(),
            lose_points: rules.ranking_rules.lose_points.into_inner(),
        })
    }

    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo> {
        self.team_info_port
            .find_enrolled_teams(season_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|t| EnrolledTeamInfo { team_id: t.team_id, team_name: t.team_name })
            .collect()
    }

    async fn find_groups(&self, season_id: &str) -> Vec<RankingGroupInfo> {
        self.group_repo
            .find_groups(season_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|g| RankingGroupInfo {
                group_id: g.group_id,
                group_name: g.group_name,
                team_ids: g.team_ids,
            })
            .collect()
    }
}
