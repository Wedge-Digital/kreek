use crate::app::match_report::ports::{ITeamDataPort, TeamInfoDto};
use crate::app::teams::ports::ITeamRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamDataAdapter {
    team_repo: Arc<dyn ITeamRepository>,
}

impl TeamDataAdapter {
    pub fn new(team_repo: Arc<dyn ITeamRepository>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl ITeamDataPort for TeamDataAdapter {
    async fn is_team_ready_to_play(&self, team_id: &str) -> Result<bool, String> {
        let team = self
            .team_repo
            .find_by_id(team_id)
            .await
            .map_err(|e| e.to_string())?;

        match team {
            Some(t) => Ok(t.game_phase
                == Some(crate::app::teams::domain::team::GamePhase::ReadyToPlay)),
            None => Ok(false),
        }
    }

    async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto> {
        let team = self.team_repo.find_by_id(team_id).await.ok()??;
        Some(TeamInfoDto {
            team_name: team.name,
            coach_name: team.coach_name,
            roster_name: team.roster_name,
        })
    }
}
