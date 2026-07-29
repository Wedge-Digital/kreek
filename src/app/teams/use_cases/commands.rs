use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{SpaceId, UserId};

pub struct DismissTeamCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub admin_id: UserId,
}

pub struct RejectEnrollmentCommand {
    pub team_id: TeamId,
}

pub struct ValidateImprovementPhaseCommand {
    pub team_id: TeamId,
}

pub struct ValidateRecruitmentPhaseCommand {
    pub team_id: TeamId,
}

pub struct ValidateDismissalsPhaseCommand {
    pub team_id: TeamId,
}
