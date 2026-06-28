use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, EntityId};
use crate::app::shared_kernel::name_vo::NameVo;
use serde::{Deserialize, Serialize};

pub type TeamName = NameVo;

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

    pub fn name(&self) -> &TeamName {
        &self.name
    }
    pub fn coach_id(&self) -> &CoachId {
        &self.coach_id
    }
    pub fn logo_url(&self) -> Option<&CloudinaryImage> {
        self.logo_url.as_ref()
    }
}
