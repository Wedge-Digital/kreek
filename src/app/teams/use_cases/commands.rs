use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{SpaceId, UserId};

#[derive(Debug)]
pub struct DismissTeamCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub admin_id: UserId,
}

#[derive(Debug)]
pub struct RejectEnrollmentCommand {
    pub team_id: TeamId,
}

#[derive(Debug)]
pub struct ValidateImprovementPhaseCommand {
    pub team_id: TeamId,
}

#[derive(Debug)]
pub struct ValidateRecruitmentPhaseCommand {
    pub team_id: TeamId,
}

#[derive(Debug)]
pub struct ValidateDismissalsPhaseCommand {
    pub team_id: TeamId,
}

// ── Panier de phase ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AddBasketPlayerCommand {
    pub team_id: TeamId,
    pub roster_line_id: String,
    pub expected_version: u32,
}

#[derive(Debug)]
pub struct AddBasketStaffCommand {
    pub team_id: TeamId,
    pub staff_type: crate::app::teams::domain::value_objects::StaffType,
    pub expected_version: u32,
}

/// Partagée par les deux phases : retirer une ligne d'un panier par son
/// identifiant est la même opération au recrutement et aux renvois. La phase
/// dit seulement quel panier ouvrir.
#[derive(Debug)]
pub struct RemoveBasketLineCommand {
    pub team_id: TeamId,
    pub phase: crate::app::teams::domain::team::GamePhase,
    pub line_id: String,
    pub expected_version: u32,
}

/// Marquer, et non retirer : le joueur reste dans l'effectif — et compte encore
/// dans le plancher des éligibles — jusqu'à la validation du lot.
#[derive(Debug)]
pub struct MarkPlayerForDismissalCommand {
    pub team_id: TeamId,
    pub player_id: crate::app::shared_kernel::bloodbowl::ids::PlayerId,
    pub expected_version: u32,
}

#[derive(Debug)]
pub struct MarkStaffForDismissalCommand {
    pub team_id: TeamId,
    pub staff_type: crate::app::teams::domain::value_objects::StaffType,
    pub expected_version: u32,
}
