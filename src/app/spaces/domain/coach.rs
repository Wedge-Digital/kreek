use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::CoachId;

pub struct Coach {
    pub id:      CoachId,
    pub name:    CoachName,
    pub profile: SpaceProfile,
    pub icon:    CoachIcon,
}

impl Coach {
    pub fn new(id: CoachId, name: CoachName, profile: SpaceProfile, icon: CoachIcon) -> Self {
        Self { id, name, profile, icon }
    }
}