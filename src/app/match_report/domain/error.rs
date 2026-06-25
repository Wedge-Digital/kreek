use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    SameTeam,
    InvalidEventSequence,
    EmptyEventStream,
    InvalidD3Roll(u8),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameTeam => write!(f, "les deux équipes doivent être différentes"),
            Self::InvalidEventSequence => write!(f, "séquence d'événements invalide"),
            Self::EmptyEventStream => write!(f, "aucun événement dans le stream"),
            Self::InvalidD3Roll(v) => write!(f, "jet de D3 invalide : {v} (attendu 1, 2 ou 3)"),
        }
    }
}
