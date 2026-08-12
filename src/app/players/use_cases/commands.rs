use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, TeamId};
use crate::app::players::domain::value_objects::{DisplayOrder, JerseyVo, PersonalName, SkillId};

pub struct PurchaseSkillCommand {
    pub player_id: PlayerId,
    pub skill_id: SkillId,
    pub mode: AcquisitionMode,
}

pub struct IncreaseStatCommand {
    pub player_id: PlayerId,
    pub stat: StatKind,
}

/// Édition de l'effectif en un lot. Les lignes absentes du lot ne sont pas
/// touchées — mais elles comptent quand même pour l'unicité du numéro et de
/// l'ordre, qui porte sur l'effectif actif entier.
pub struct UpdateRosterCommand {
    pub team_id: TeamId,
    pub rows: Vec<RosterRowCommand>,
}

pub struct RosterRowCommand {
    pub player_id: PlayerId,
    /// `None` efface le nom : la lecture retombe alors sur le nom de poste.
    pub personal_name: Option<PersonalName>,
    /// `None` retire le numéro — un joueur peut n'en porter aucun.
    pub jersey: Option<JerseyVo>,
    /// Toujours fourni : le rang vient de la position de la ligne dans le
    /// formulaire, il n'y a donc jamais d'ordre « non renseigné » à la saisie.
    pub display_order: DisplayOrder,
}
