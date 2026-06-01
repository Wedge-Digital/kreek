use std::fmt;
use crate::app::teams::domain::team::{GamePhase, ParticipationStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    InvalidTransition { from: ParticipationStatus, to: ParticipationStatus },
    NotEnrolled,
    AlreadyDismissed,
    WrongGamePhase(Option<GamePhase>),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } =>
                write!(f, "transition invalide : {from:?} → {to:?}"),
            Self::NotEnrolled      => write!(f, "équipe non inscrite"),
            Self::AlreadyDismissed => write!(f, "équipe déjà renvoyée"),
            Self::WrongGamePhase(p) => write!(f, "phase de jeu incorrecte : {p:?}"),
        }
    }
}
