use std::fmt;

/// Erreurs d'invariant du domaine `competitions`.
///
/// `Display` est implémenté à la main, comme dans les autres BCs
/// (`players`, `teams`, `match_report`) : le projet n'utilise pas `thiserror`.
/// Le message est directement exploitable comme corps de réponse 422, le
/// formulaire de règles affichant la réponse telle quelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyTiebreakConfig,
    NoActiveTiebreaker,
    DuplicateTiebreakCode { code: String },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTiebreakConfig => {
                write!(f, "La configuration de départage est vide.")
            }
            Self::NoActiveTiebreaker => {
                write!(f, "Au moins un critère de départage doit être actif.")
            }
            Self::DuplicateTiebreakCode { code } => {
                write!(
                    f,
                    "Le critère de départage « {code} » est présent plusieurs fois."
                )
            }
        }
    }
}
