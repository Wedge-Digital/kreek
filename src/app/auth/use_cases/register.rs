use super::super::domain::{
    coach_name::CoachName,
    email::Email,
    error::AuthDomainError,
};
use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::auth::use_cases::commands::RegisterCommand;
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::user::User;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use std::fmt;

#[derive(Debug)]
pub enum RegisterError {
    PasswordMismatch,
    PasswordTooShort,
    InvalidCoachName(AuthDomainError),
    InvalidEmail(AuthDomainError),
    CoachNameAlreadyTaken,
    EmailAlreadyTaken,
    PasswordHashError,
    Database(String),
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterError::PasswordMismatch        => write!(f, "Les mots de passe ne correspondent pas"),
            RegisterError::PasswordTooShort        => write!(f, "Le mot de passe doit contenir au moins 8 caractères"),
            RegisterError::InvalidCoachName(e)     => write!(f, "{}", e),
            RegisterError::InvalidEmail(e)         => write!(f, "{}", e),
            RegisterError::CoachNameAlreadyTaken   => write!(f, "Ce nom de coach est déjà utilisé"),
            RegisterError::EmailAlreadyTaken       => write!(f, "Cette adresse email est déjà utilisée"),
            RegisterError::PasswordHashError       => write!(f, "Erreur lors du chiffrement du mot de passe"),
            RegisterError::Database(msg)           => write!(f, "Erreur interne : {}", msg),
        }
    }
}

impl From<RepositoryError> for RegisterError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::CoachNameAlreadyTaken => RegisterError::CoachNameAlreadyTaken,
            RepositoryError::EmailAlreadyTaken     => RegisterError::EmailAlreadyTaken,
            RepositoryError::Database(msg)         => RegisterError::Database(msg),
        }
    }
}

pub async fn execute(
    cmd: RegisterCommand,
    repo: &dyn IUserRepository,
) -> Result<(), RegisterError> {
    if cmd.password != cmd.password_confirm {
        return Err(RegisterError::PasswordMismatch);
    }
    if cmd.password.len() < 8 {
        return Err(RegisterError::PasswordTooShort);
    }

    let coach_name = CoachName::new(&cmd.coach_name)
        .map_err(RegisterError::InvalidCoachName)?;
    let email = Email::new(&cmd.email)
        .map_err(RegisterError::InvalidEmail)?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(cmd.password.as_bytes(), &salt)
        .map_err(|_| RegisterError::PasswordHashError)?
        .to_string();

    let user = User::new(UserId::new(), coach_name, email, password_hash);

    repo.create(&user).await.map_err(RegisterError::from)
}
