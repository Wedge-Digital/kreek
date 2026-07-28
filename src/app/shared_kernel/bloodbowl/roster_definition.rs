use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::bloodbowl::ids::RosterId;

/// DTO de lecture (query) — `shared_kernel` ne dépend d'aucun BC applicatif,
/// donc `name` est un `String` nu plutôt que le `RosterName` validé de
/// `team_creation` (cf. règle CQRS du CLAUDE.md : primitifs autorisés côté
/// lecture, jamais côté écriture domaine).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RosterDefinition {
    pub id: RosterId,
    pub name: String,
}