use crate::app::teams::domain::team::{GamePhase, ParticipationStatus};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    InvalidTransition {
        from: ParticipationStatus,
        to: ParticipationStatus,
    },
    NotEnrolled,
    AlreadyDismissed,
    WrongGamePhase(Option<GamePhase>),
    StaffTypeNotBuyable,
    StaffTypeNotDismissable,
    InsufficientStaff,
    /// Aucune séquence d'après-match à défaire pour ce rapport.
    NoPostMatchToRevert,
    /// L'équipe n'est pas en saisie sur ce rapport — soit elle n'en saisit
    /// aucun, soit elle en saisit un autre.
    NotReportingThisMatch,
    InsufficientTreasury,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "transition invalide : {from:?} → {to:?}")
            }
            Self::NotEnrolled => write!(f, "équipe non inscrite"),
            Self::AlreadyDismissed => write!(f, "équipe déjà renvoyée"),
            Self::WrongGamePhase(p) => write!(f, "phase de jeu incorrecte : {p:?}"),
            Self::StaffTypeNotBuyable => write!(
                f,
                "ce type de staff ne peut pas être acheté en phase de recrutement"
            ),
            Self::StaffTypeNotDismissable => write!(f, "ce type de staff ne peut pas être renvoyé"),
            Self::InsufficientStaff => write!(f, "quantité insuffisante de ce staff"),
            Self::NoPostMatchToRevert => {
                write!(f, "aucune séquence d'après-match à défaire pour ce rapport")
            }
            Self::NotReportingThisMatch => {
                write!(f, "l'équipe n'est pas en saisie sur ce rapport de match")
            }
            Self::InsufficientTreasury => write!(f, "trésorerie insuffisante"),
        }
    }
}
