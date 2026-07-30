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

    // ── Levées par les paniers de phase (cartes 262 et 267) ───────────────
    // Déclarées ici parce que `DomainError` est l'erreur du BC, pas celle d'un
    // agrégat : les paniers la partagent. Elles naissent donc sans appelant.
    /// Plafond de 16 joueurs atteint.
    MaxPlayersReached,
    /// Le quota du poste dans le roster est atteint (`max_quantity`).
    PositionQuotaReached,
    /// Limite de cumul entre postes dépassée — « pas plus de 3 gros joueurs ».
    CrossLimitExceeded,
    /// Le poste demandé n'appartient pas au roster de l'équipe.
    PositionNotInRoster,
    /// Le roster de l'équipe n'a pas droit à ce type de staff.
    StaffNotAllowedForRoster,
    /// Le quota de ce staff est atteint (`max_quantity` du catalogue).
    StaffQuotaReached,
    /// Le renvoi ferait passer l'effectif sous les onze joueurs éligibles.
    EligibleFloorReached,
    /// La ligne visée n'existe pas dans le panier.
    BasketLineNotFound,
    /// Le joueur visé n'appartient pas à l'effectif de cette équipe.
    PlayerNotInSquad,
    /// Le joueur est déjà marqué pour renvoi. Sans cette garde, une seconde
    /// ligne le compterait deux fois dans le plancher des éligibles, et le lot
    /// émettrait deux `PlayerDismissed` pour un même joueur.
    PlayerAlreadyMarked,
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
            Self::MaxPlayersReached => write!(f, "effectif complet : 16 joueurs maximum"),
            Self::PositionQuotaReached => {
                write!(f, "quota atteint pour ce poste dans le roster")
            }
            Self::CrossLimitExceeded => write!(f, "limite combinée de postes dépassée"),
            Self::PositionNotInRoster => write!(f, "ce poste n'appartient pas au roster"),
            Self::StaffNotAllowedForRoster => {
                write!(f, "ce roster n'a pas droit à ce type de staff")
            }
            Self::StaffQuotaReached => write!(f, "quota atteint pour ce type de staff"),
            Self::EligibleFloorReached => write!(
                f,
                "l'effectif ne peut pas descendre sous onze joueurs éligibles"
            ),
            Self::BasketLineNotFound => write!(f, "ligne introuvable dans le panier"),
            Self::PlayerNotInSquad => write!(f, "ce joueur n'appartient pas à l'effectif"),
            Self::PlayerAlreadyMarked => write!(f, "ce joueur est déjà marqué pour renvoi"),
        }
    }
}
