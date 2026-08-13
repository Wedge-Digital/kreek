use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId, TeamId};
use crate::app::players::domain::value_objects::{
    BasketLineId, CustomisationId, DisplayOrder, JerseyVo, KpoDelta, PersonalName, SkillId,
    SppAmount, StatCrans,
};

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

// ── Customisation ─────────────────────────────────────────────────────────────
//
// `expected_version` porte la garde d'écriture concurrente. Elle vient du
// **formulaire**, cuite dans les `hx-vals` du panneau au moment de son rendu :
// le panneau étant re-rendu après chaque mutation, la version qu'il porte est
// toujours celle d'après la dernière écriture.
//
// Le piège de la carte 264 n'est pas de faire circuler la version, c'est de
// reposer sur l'agrégat celle que `save` vient de rendre.

pub struct AddCustomisationSkillCommand {
    pub player_id: PlayerId,
    pub skill_id: SkillId,
    pub expected_version: u32,
}

pub struct AddCustomisationStatCommand {
    pub player_id: PlayerId,
    pub stat: StatKind,
    /// En **qualité du joueur** : `+1` améliore, `-1` dégrade. La traduction en
    /// offset brut appartient au domaine, seul détenteur de la table des
    /// directions.
    pub crans: StatCrans,
    pub expected_version: u32,
}

pub struct AdjustCustomisationPriceCommand {
    pub player_id: PlayerId,
    pub delta: KpoDelta,
    pub expected_version: u32,
}

pub struct AddCustomisationSppCommand {
    pub player_id: PlayerId,
    pub amount: SppAmount,
    pub expected_version: u32,
}

pub struct RemoveCustomisationLineCommand {
    pub player_id: PlayerId,
    pub line_id: BasketLineId,
    pub expected_version: u32,
}

/// Les identifiants de customisation sont **portés par la commande**, engendrés
/// par le handler : ni le domaine ni le use case ne doivent tirer d'aléatoire,
/// sous peine de devenir intestables. Un par ligne à appliquer.
///
/// D'où `expected_version`, que la validation porte comme les cinq mutations.
/// Le handler doit compter les lignes pour savoir combien d'identifiants
/// engendrer, et il les compte sur une lecture antérieure à celle du use case.
/// Sans garde de version, un panier modifié entre les deux lectures ferait
/// diverger les comptes et sortir `IdentifiantsManquants` — un code qui annonce
/// un bug d'appelant pour ce qui n'est qu'une écriture concurrente.
///
/// Avec la garde, le contenu ne peut plus changer sans que la version change :
/// la course retombe sur `ConcurrentWrite`, chemin déjà silencieux et déjà
/// compris, et `IdentifiantsManquants` redevient ce qu'il prétend être.
pub struct ValidateCustomisationCommand {
    pub player_id: PlayerId,
    pub author: String, // arch:ok nom d'affichage du commissaire
    pub customisation_ids: Vec<CustomisationId>,
    pub expected_version: u32,
}

pub struct CancelCustomisationCommand {
    pub player_id: PlayerId,
}
