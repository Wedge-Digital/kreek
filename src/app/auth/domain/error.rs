use std::fmt;

#[derive(Debug)]
pub enum AuthDomainError {
    CoachNameEmpty,
    CoachNameTooLong,
    CoachNameInvalidChars,
    EmailInvalid,
    EmailTooLong,
}

impl fmt::Display for AuthDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthDomainError::CoachNameEmpty => write!(f, "Le nom du coach ne peut pas être vide"),
            AuthDomainError::CoachNameTooLong => {
                write!(f, "Le nom du coach ne peut pas dépasser 50 caractères")
            }
            AuthDomainError::CoachNameInvalidChars => write!(
                f,
                "Le nom du coach ne peut contenir que des lettres, chiffres et espaces"
            ),
            AuthDomainError::EmailInvalid => write!(f, "L'adresse email est invalide"),
            AuthDomainError::EmailTooLong => {
                write!(f, "L'adresse email ne peut pas dépasser 255 caractères")
            }
        }
    }
}

impl std::error::Error for AuthDomainError {}
