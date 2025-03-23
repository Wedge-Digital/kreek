use serde::{Deserialize, Serialize};
use crate::app::global_types::global_type::EntityId;

pub type TeamName = String;
pub type CoachId = EntityId;

pub type UserId = EntityId;

pub type TeamId = EntityId;
pub type CloudinaryImage = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseTeamInfo {
    name: TeamName,
    coach_id: CoachId,
    logo_url: Option<CloudinaryImage>,
}

impl BaseTeamInfo {
    pub fn new(name: TeamName, coach_id: CoachId, logo_url: Option<CloudinaryImage>) -> Self {
        BaseTeamInfo {
            name,
            coach_id,
            logo_url,
        }
    }
}