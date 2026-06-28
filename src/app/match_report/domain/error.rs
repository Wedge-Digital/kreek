use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    SameTeam,
    InvalidEventSequence,
    EmptyEventStream,
    InvalidD3Roll(u8),
    BudgetExceeded { spent: u32, budget: u32 },
    MaxQtyExceeded { uid: String, qty: u8, max_qty: u8 },
    StarPlayerLimitExceeded,
    StarPlayerConflict { uid: String },
    TeamValuesNotRecorded,
    InducementsAlreadyRecorded,
    InvalidTurn(u8),
    ActionNotFound(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameTeam => write!(f, "les deux équipes doivent être différentes"),
            Self::InvalidEventSequence => write!(f, "séquence d'événements invalide"),
            Self::EmptyEventStream => write!(f, "aucun événement dans le stream"),
            Self::InvalidD3Roll(v) => write!(f, "jet de D3 invalide : {v} (attendu 1, 2 ou 3)"),
            Self::BudgetExceeded { spent, budget } => {
                write!(f, "budget dépassé : {spent} kPo dépensés pour {budget} kPo disponibles")
            }
            Self::MaxQtyExceeded { uid, qty, max_qty } => {
                write!(f, "quantité invalide pour {uid} : {qty} (max {max_qty})")
            }
            Self::StarPlayerLimitExceeded => write!(f, "maximum 2 star players par équipe"),
            Self::StarPlayerConflict { uid } => {
                write!(f, "star player {uid} déjà recruté par l'équipe adverse")
            }
            Self::TeamValuesNotRecorded => write!(f, "les team values ne sont pas encore enregistrées"),
            Self::InducementsAlreadyRecorded => write!(f, "les inducements ont déjà été enregistrés pour cette équipe"),
            Self::InvalidTurn(v) => write!(f, "tour invalide : {v} (attendu 1..=16)"),
            Self::ActionNotFound(id) => write!(f, "action introuvable : {id}"),
        }
    }
}
