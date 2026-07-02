use crate::app::match_report::ports::{ITeamDataPort, JournalierPositionDto, RosterPositionDto, TeamInfoDto};
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::teams::ports::ITeamRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RefTeamDataAdapter {
    team_repo: Arc<dyn ITeamRepository>,
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl RefTeamDataAdapter {
    pub fn new(
        team_repo: Arc<dyn ITeamRepository>,
        reference_repo: Arc<dyn IReferenceRepository>,
    ) -> Self {
        Self { team_repo, reference_repo }
    }
}

#[async_trait]
impl ITeamDataPort for RefTeamDataAdapter {
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
            team_name:       team.name.to_string(),
            coach_name:      team.coach_name,
            roster_name:     team.roster_name.to_string(),
            roster_id:       team.roster_id.to_string(),
            logo_url:        team.logo_url,
            dedicated_fans:  team.dedicated_fans.into_inner() as u32,
        })
    }

    async fn find_team_value(&self, team_id: &str) -> Option<u32> {
        let team = self.team_repo.find_by_id(team_id).await.ok()??;
        Some(team.team_value.0)
    }

    async fn find_team_treasury(&self, team_id: &str) -> Option<u32> {
        let team = self.team_repo.find_by_id(team_id).await.ok()??;
        Some(team.treasury.0)
    }

    async fn find_journalier_position(&self, team_id: &str) -> Option<JournalierPositionDto> {
        let team = self.team_repo.find_by_id(team_id).await.ok()??;
        let roster_id = team.roster_id.to_string();
        let ref_team = self.reference_repo.find_team_by_uid(&roster_id)?;
        let pos = ref_team.available_players.iter().find(|p| p.is_journalier)?;
        Some(JournalierPositionDto {
            position_uid: pos.uid.clone(),
            position_name: pos.position_name.clone(),
        })
    }

    async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto> {
        let Ok(Some(team)) = self.team_repo.find_by_id(team_id).await else { return vec![]; };
        let roster_id = team.roster_id.to_string();
        let Some(ref_team) = self.reference_repo.find_team_by_uid(&roster_id) else { return vec![]; };
        ref_team
            .available_players
            .iter()
            .map(|p| RosterPositionDto {
                position_uid:  p.uid.clone(),
                position_name: p.position_name.clone(),
                base_cost:     p.cost,
                max_qty:       p.max_quantity,
                is_journalier: p.is_journalier,
            })
            .collect()
    }
}
