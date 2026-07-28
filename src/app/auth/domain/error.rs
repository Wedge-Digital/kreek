use crate::app::shared_kernel::coach_name::CoachNameError;
use crate::app::shared_kernel::email::EmailError;
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

// Les deux conversions vivaient dans `shared_kernel` : le socle dont tout le
// monde dépend importait l'erreur d'un BC, et aucun ordre de copie ne
// permettait alors de sortir `auth` du projet. Elles sont ici, à côté du type
// qu'elles construisent. Le crate étant unique, aucun site d'appel ne change :
// les `?` des use cases convertissent exactement comme avant.
impl From<CoachNameError> for AuthDomainError {
    fn from(e: CoachNameError) -> Self {
        match e {
            CoachNameError::NotEmptyViolated => AuthDomainError::CoachNameEmpty,
            CoachNameError::LenCharMaxViolated => AuthDomainError::CoachNameTooLong,
            CoachNameError::RegexViolated => AuthDomainError::CoachNameInvalidChars,
        }
    }
}

impl From<EmailError> for AuthDomainError {
    fn from(e: EmailError) -> Self {
        match e {
            EmailError::LenCharMaxViolated => AuthDomainError::EmailTooLong,
            EmailError::RegexViolated => AuthDomainError::EmailInvalid,
        }
    }
}
