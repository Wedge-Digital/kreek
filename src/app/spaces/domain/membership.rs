//! Ce qu'une modification d'appartenance produit, et comment elle échoue.
//!
//! Les trois types vivent ensemble parce qu'ils décrivent la même chose : le
//! résultat d'une commande de `Space` sur ses membres.

use crate::app::spaces::domain::domain_event::SpacesDomainEvent;
use nutype::nutype;

/// Le nombre d'administrateurs d'un espace, **après** une opération réussie.
///
/// Le type porte l'invariant : zéro administrateur est un état qu'un espace n'a
/// pas le droit d'avoir, et le type refuse de le représenter.
///
/// Ce n'est sûr que parce qu'il n'est construit **que sur le chemin de succès**
/// d'une commande. Les trois cas se vérifient un par un : une promotion
/// augmente le compte, une rétrogradation et un retrait sont refusés s'ils
/// amèneraient à zéro. Après succès, il vaut donc toujours au moins un.
///
/// Il n'est **jamais** construit au chargement de l'agrégat. Un espace hérité
/// qui se retrouverait sans administrateur doit continuer à se charger sans
/// erreur : refuser de lire une donnée existante la rendrait inaccessible au
/// lieu de la réparer, et ses opérations de rôle échoueront proprement — ce qui
/// est le bon symptôme.
#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct NombreAdministrateurs(usize);

/// Le résultat d'une commande d'appartenance.
///
/// Le compte voyage **à côté** de l'événement, jamais dedans. Les événements
/// sont persistés pour toujours par `event_log_feeder` : un `administrateurs: 3`
/// inscrit au journal serait un instantané que plus rien ne rendra vrai, et qui
/// inviterait un lecteur futur à s'y fier. Un événement dit ce qui s'est passé,
/// pas l'état qui en résulte.
///
/// Il n'y a pas non plus de lecture publique du compte sur l'agrégat : ce serait
/// une invitation à reprendre au-dehors une décision qui lui appartient. Le
/// compte n'existe que comme produit d'une opération réussie, c'est-à-dire au
/// seul instant où il est vrai.
#[derive(Debug)]
pub struct ChangementDAppartenance {
    /// Absent quand la commande n'a **rien changé** — reposter le rôle courant
    /// réussit sans qu'aucun fait ne se soit produit, et le journal ne doit pas
    /// enregistrer un changement qui n'a pas eu lieu.
    pub evenement: Option<SpacesDomainEvent>,
    pub administrateurs: NombreAdministrateurs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceMembershipError {
    /// L'invariant : un espace a toujours au moins un administrateur.
    DernierAdministrateur,
    /// On ne modifie pas son propre rôle, on ne se retire pas soi-même.
    ActeurEstLaCible,
    /// La cible n'appartient pas à l'espace.
    PasMembre,
    /// La cible appartient déjà à l'espace.
    ///
    /// Sans cette règle, le doublon serait refusé par la clé primaire composite
    /// de `spaces__user_space` — une règle métier rendue par une erreur SQL
    /// brute, illisible et intraduisible en 409.
    DejaMembre,
}
