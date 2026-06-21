use crate::app::shared_kernel::common_types::{SpaceId, UserId};
use crate::app::shared_kernel::team::TeamId;

pub struct DismissTeamCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub admin_id: UserId,
}

pub struct RejectEnrollmentCommand {
    pub team_id: TeamId,
}
