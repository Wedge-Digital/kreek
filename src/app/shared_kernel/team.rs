use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, EntityId};
use nutype::nutype;
use serde::{Deserialize, Serialize};

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[a-zA-Z0-9 ]+$"),
    derive(Debug, Clone, Serialize, Deserialize)
)]
pub struct TeamName(String);

pub type TeamId = EntityId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
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