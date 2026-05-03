use super::super::domain::{
    coach_name::CoachName,
    email::Email,
    error::AuthDomainError,
};
use super::super::ports::{IUserRepository, RepositoryError};
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

pub struct RegisterCommand {
    pub coach_name:       String,
    pub email:            String,
    pub password:         String,
    pub password_confirm: String,
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
) -> Result<(), Vec<RegisterError>> {
    let mut errors: Vec<RegisterError> = Vec::new();

    // --- validation : tous les champs sont vérifiés sans court-circuit ---

    if cmd.password != cmd.password_confirm {
        errors.push(RegisterError::PasswordMismatch);
    }
    if cmd.password.len() < 8 {
        errors.push(RegisterError::PasswordTooShort);
    }

    let coach_name = match CoachName::try_new(&cmd.coach_name) {
        Ok(v)  => Some(v),
        Err(e) => { errors.push(RegisterError::InvalidCoachName(e.into())); None }
    };
    let email = match Email::try_new(&cmd.email) {
        Ok(v)  => Some(v),
        Err(e) => { errors.push(RegisterError::InvalidEmail(e.into())); None }
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    // --- à partir d'ici les valeurs sont garanties valides ---

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(cmd.password.as_bytes(), &salt)
        .map_err(|_| vec![RegisterError::PasswordHashError])?
        .to_string();

    let user = User::new(
        UserId::new(),
        coach_name.unwrap(),
        email.unwrap(),
        password_hash,
    );

    repo.create(&user).await.map_err(|e| vec![RegisterError::from(e)])
}
