use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{SpaceId, UserId};
use crate::app::teams::domain::team::OverridablePhase;
use crate::app::teams::domain::treasury::MovementDirection;
use crate::app::teams::domain::value_objects::{AdjustmentAmount, AdjustmentNote, OverrideReason};

/// Un commissaire d'espace crédite ou débite la caisse d'une équipe.
///
/// **`admin_name` voyage avec la commande** et finira dans l'événement : c'est
/// le nom au moment de l'acte, celui que le relevé affichera des mois plus
/// tard. Le résoudre à la lecture demanderait un port et réécrirait l'histoire
/// le jour où un coach se renomme.
///
/// Le `Debug` sert la journalisation — le use case est instrumenté
/// `fields(cmd = ?cmd)` — et rien ici n'est un secret : un motif est fait pour
/// être lu.
#[derive(Debug)]
pub struct AdjustTreasuryCommand {
    pub team_id: TeamId,
    pub direction: MovementDirection,
    pub amount: AdjustmentAmount,
    pub note: AdjustmentNote,
    pub admin_id: UserId,
    pub admin_name: CoachName,
}

/// Un admin ouvre à la main l'une des trois phases d'après-match (carte 577).
///
/// Comme pour l'ajustement de trésorerie, `admin_name` voyage avec la commande :
/// c'est le nom au moment de l'acte. Le motif est facultatif — un champ vide
/// donne `None`. Rien ici n'est un secret.
#[derive(Debug)]
pub struct OpenPhaseOverrideCommand {
    pub team_id: TeamId,
    pub phase: OverridablePhase,
    pub reason: Option<OverrideReason>,
    pub admin_id: UserId,
    pub admin_name: CoachName,
}

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

/// Garder un journalier. **Un `player_id`, pas une ligne de roster** : on
/// désigne un homme qui existe déjà, pas un poste à pourvoir.
#[derive(Debug)]
pub struct AddBasketJourneymanCommand {
    pub team_id: TeamId,
    pub player_id: PlayerId,
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
