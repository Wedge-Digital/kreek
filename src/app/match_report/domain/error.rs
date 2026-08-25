use crate::app::match_report::domain::value_objects::CorrectionBlocker;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    SameTeam,
    InvalidEventSequence,
    EmptyEventStream,
    InvalidD3Roll(u8),
    BudgetExceeded {
        spent: u32,
        budget: u32,
    },
    MaxQtyExceeded {
        uid: String,
        qty: u8,
        max_qty: u8,
    },
    /// Un achat dont aucune spécification ne porte l'uid.
    ///
    /// C'est une incohérence d'appelant, jamais une donnée à ignorer : le coup
    /// de pouce a été facturé au coach, et le filtrer le faisait disparaître
    /// sans un mot (carte 406).
    UnknownInducement {
        uid: String,
    },
    StarPlayerLimitExceeded,
    StarPlayerConflict {
        uid: String,
    },
    TeamValuesNotRecorded,
    InvalidTurn(u8),
    ActionNotFound(String),
    TooManyMercenaries {
        requested: u8,
        max: u8,
    },
    CorrectionNotAllowed(CorrectionBlocker),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameTeam => write!(f, "les deux équipes doivent être différentes"),
            Self::InvalidEventSequence => write!(f, "séquence d'événements invalide"),
            Self::EmptyEventStream => write!(f, "aucun événement dans le stream"),
            Self::InvalidD3Roll(v) => write!(f, "jet de D3 invalide : {v} (attendu 1, 2 ou 3)"),
            Self::BudgetExceeded { spent, budget } => {
                write!(
                    f,
                    "budget dépassé : {spent} kPo dépensés pour {budget} kPo disponibles"
                )
            }
            Self::MaxQtyExceeded { uid, qty, max_qty } => {
                write!(f, "quantité invalide pour {uid} : {qty} (max {max_qty})")
            }
            Self::UnknownInducement { uid } => {
                write!(f, "coup de pouce inconnu du tier : {uid}")
            }
            Self::StarPlayerLimitExceeded => write!(f, "maximum 2 star players par équipe"),
            Self::StarPlayerConflict { uid } => {
                write!(f, "star player {uid} déjà recruté par l'équipe adverse")
            }
            Self::TeamValuesNotRecorded => {
                write!(f, "les team values ne sont pas encore enregistrées")
            }
            Self::InvalidTurn(v) => write!(f, "tour invalide : {v} (attendu 1..=16)"),
            Self::ActionNotFound(id) => write!(f, "action introuvable : {id}"),
            Self::TooManyMercenaries { requested, max } => {
                write!(f, "trop de mercenaires : {requested} demandés, max {max}")
            }
            // Sans nom d'équipe : le domaine ne connaît que le camp concerné.
            Self::CorrectionNotAllowed(blocker) => match blocker {
                CorrectionBlocker::SppAlreadySpent { .. } => {
                    write!(f, "correction impossible : des SPP ont déjà été dépensés")
                }
                CorrectionBlocker::PhaseAdvanced { .. } => {
                    write!(
                        f,
                        "correction impossible : une équipe a quitté la phase d'amélioration"
                    )
                }
                CorrectionBlocker::EligibilityUnknown => {
                    write!(
                        f,
                        "correction impossible : l'éligibilité n'a pas pu être vérifiée"
                    )
                }
            },
        }
    }
}
