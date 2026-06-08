use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    // Réservé pour les futures règles métier (amélioration, renvoi, etc.)
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "erreur domaine players")
    }
}
