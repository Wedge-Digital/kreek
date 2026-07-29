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

// ── Panier de phase ───────────────────────────────────────────────────────────

pub struct AddBasketPlayerCommand {
    pub team_id: TeamId,
    pub roster_line_id: String,
    pub expected_version: u32,
}

pub struct AddBasketStaffCommand {
    pub team_id: TeamId,
    pub staff_type: crate::app::teams::domain::value_objects::StaffType,
    pub expected_version: u32,
}

/// Partagée par les deux phases : retirer une ligne d'un panier par son
/// identifiant est la même opération au recrutement et aux renvois. La phase
/// dit seulement quel panier ouvrir.
pub struct RemoveBasketLineCommand {
    pub team_id: TeamId,
    pub phase: crate::app::teams::domain::team::GamePhase,
    pub line_id: String,
    pub expected_version: u32,
}
