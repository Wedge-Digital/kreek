//! Le bouton de saisie d'un match, tel que « Mes matchs » l'affiche (carte 677).
//!
//! `competitions` ne décide pas lequel : c'est `match_report` qui le sait, et
//! qui le rend par le port `IMatchEntryPort`. Ce type n'est que la forme sous
//! laquelle la réponse entre dans ce BC.

/// Ce que le coach peut faire sur son prochain match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchEntryKind {
    /// Le brouillon n'est pas commencé.
    Start,
    /// La saisie est commencée, et le coach y écrit.
    Continue,
    /// L'adversaire saisit tout le match : le coach le suit en lecture seule.
    Follow,
}
