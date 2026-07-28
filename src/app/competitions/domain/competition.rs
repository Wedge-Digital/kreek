use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;

pub struct Competition {
    pub id: CompetitionId,
    pub space_id: SpaceId,
    pub name: CompetitionName,
    pub logo: CloudinaryImage,
    pub admin_ids: Vec<CoachId>,
}

impl Competition {
    pub fn new(
        space_id: SpaceId,
        name: CompetitionName,
        logo: CloudinaryImage,
        admin_ids: Vec<CoachId>,
    ) -> Self {
        Self {
            id: CompetitionId::new(),
            space_id,
            name,
            logo,
            admin_ids,
        }
    }
}
