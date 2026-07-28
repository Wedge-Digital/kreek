use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::CoachId;

pub struct Coach {
    pub id: CoachId,
    pub name: CoachName,
    pub profile: SpaceProfile,
    pub icon: CoachIcon,
}

impl Coach {
    pub fn new(id: CoachId, name: CoachName, profile: SpaceProfile, icon: CoachIcon) -> Self {
        Self {
            id,
            name,
            profile,
            icon,
        }
    }
}
