use crate::app::players::ports::{IPlayerRosterPort, TeamRosterInfoDto};
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::ports::ITeamRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamRosterAdapter {
    team_repo: Arc<dyn ITeamRepository>,
}

impl TeamRosterAdapter {
    pub fn new(team_repo: Arc<dyn ITeamRepository>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl IPlayerRosterPort for TeamRosterAdapter {
    async fn find_team_info(&self, team_id: &str) -> Option<TeamRosterInfoDto> {
        let team = self.team_repo.find_by_id(team_id).await.ok()??;
        Some(TeamRosterInfoDto {
            team_name: team.name.to_string(),
            coach_id: team.coach_id.to_string(),
            competition_id: team.competition_id.map(|id| id.to_string()),
            in_player_improvement_phase: team.game_phase == Some(GamePhase::PlayerImprovement),
        })
    }
}
