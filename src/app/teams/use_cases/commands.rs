use crate::app::shared_kernel::common_types::{SpaceId, UserId};
use crate::app::shared_kernel::team::TeamId;

pub struct DismissTeamCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub admin_id: UserId,
}

pub struct ApproveEnrollmentCommand {
    pub team_id: TeamId,
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub season_name: String,
}

pub struct RejectEnrollmentCommand {
    pub team_id: TeamId,
}
