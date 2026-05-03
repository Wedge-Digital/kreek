use std::fmt;

#[derive(Debug)]
pub enum AuthDomainError {
    CoachNameEmpty,
    CoachNameTooLong,
    EmailInvalid,
    EmailTooLong,
}

impl fmt::Display for AuthDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthDomainError::CoachNameEmpty    => write!(f, "Le nom du coach ne peut pas être vide"),
            AuthDomainError::CoachNameTooLong  => write!(f, "Le nom du coach ne peut pas dépasser 100 caractères"),
            AuthDomainError::EmailInvalid      => write!(f, "L'adresse email est invalide"),
            AuthDomainError::EmailTooLong      => write!(f, "L'adresse email ne peut pas dépasser 255 caractères"),
        }
    }
}

impl std::error::Error for AuthDomainError {}
