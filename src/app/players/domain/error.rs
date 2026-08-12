use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    SkillAlreadyAcquired,
    InsufficientSpp,
    PlayerNotActive,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SkillAlreadyAcquired => write!(f, "compétence déjà possédée"),
            Self::InsufficientSpp => write!(f, "SPP insuffisants"),
            Self::PlayerNotActive => write!(f, "joueur non actif"),
        }
    }
}
