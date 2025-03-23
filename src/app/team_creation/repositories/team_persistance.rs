use crate::app::global_types::global_type::{EntityId};
use crate::app::team_creation::team_draft::DraftTeam;

pub trait TeamPersistance {
    async fn save(&mut self, team: &DraftTeam) -> Result<(), String>;

    async fn update(&mut self, team: DraftTeam) -> Result<(), String>;

    async fn add_or_update(&mut self, team: DraftTeam) -> Result<(), String>;

    async fn get_by_id(&self, id: EntityId) -> Option<DraftTeam>;

    async fn delete(&mut self, team: DraftTeam) -> Result<(), String>;

    async fn get_all(&self) -> Vec<DraftTeam>;
}