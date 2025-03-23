use crate::app::global_types::global_type::{Entity, EntityId};
use crate::app::team_creation::repositories::team_persistance::TeamPersistance;
use crate::app::team_creation::team_draft::DraftTeam;

pub struct InMemoryTeamPersistance {
    teams: Vec<DraftTeam>
}

impl InMemoryTeamPersistance {
    pub fn new() -> Self {
        return InMemoryTeamPersistance{teams: vec![]};
    }
}

impl TeamPersistance for InMemoryTeamPersistance {

    async fn save(&mut self, team: &DraftTeam) -> Result<(), String> {
        self.teams.push(team.clone());
        return Ok(());
    }

    async fn update(&mut self, team: DraftTeam) -> Result<(), String> {
        todo!()
    }

    async fn add_or_update(&mut self, team: DraftTeam) -> Result<(), String> {
        todo!()
    }

    async fn get_by_id(&self, id: EntityId) -> Option<DraftTeam> {
        for team in &self.teams {
            if team.get_id() == id {
                return Some(team.clone());
            }
        }
        return None;
    }

    async fn delete(&mut self, team_to_delete: DraftTeam) -> Result<(), String> {
        for (i, team) in self.teams.iter().enumerate() {
            if team == &team_to_delete {
                self.teams.remove(i);
                return Ok(());
            }
        }
        return Err("Team not found".to_string());
    }

    async fn get_all(&self) -> Vec<DraftTeam> {
        return self.teams.clone();
    }
}