use crate::app::competitions::ports::{ITeamInfoPort, TeamInfoDto};
use crate::app::teams::ports::ITeamRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamInfoAdapter {
    team_repo: Arc<dyn ITeamRepository>,
}

impl TeamInfoAdapter {
    pub fn new(team_repo: Arc<dyn ITeamRepository>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl ITeamInfoPort for TeamInfoAdapter {
    async fn find_enrolled_teams(&self, season_id: &str) -> Result<Vec<TeamInfoDto>, String> {
        let rows = self
            .team_repo
            .find_enrolled_for_season(season_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|r| TeamInfoDto {
                team_id: r.team_id,
                team_name: r.team_name,
                coach_id: r.coach_id,
                coach_name: r.coach_name,
                roster_name: r.roster_name,
                logo_url: r.logo_url,
            })
            .collect())
    }

    async fn find_team_names(&self, team_ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
        let mut out = Vec::with_capacity(team_ids.len());
        for id in team_ids {
            if let Ok(Some(team)) = self.team_repo.find_by_id(id).await {
                out.push(TeamInfoDto {
                    team_id:     id.clone(),
                    team_name:   team.name.to_string(),
                    coach_id:    team.coach_id.to_string(),
                    coach_name:  team.coach_name.clone(),
                    roster_name: team.roster_name.to_string(),
                    logo_url:    team.logo_url.clone(),
                });
            }
        }
        Ok(out)
    }
}
