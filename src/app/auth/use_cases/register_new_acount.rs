use super::super::domain::error::AuthDomainError;
use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::auth::domain::domain_event::AuthDomainEvent::AccountCreated;
use crate::app::auth::domain::user::User;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::{EventId, UserId};
use crate::app::shared_kernel::identity::secret::Secret;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
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

#[derive(Debug)]
pub struct RegisterCommand {
    pub coach_name: String,
    pub email: Secret<String>,
    pub password: Secret<String>,
    pub password_confirm: Secret<String>,
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterError::PasswordMismatch => write!(f, "Les mots de passe ne correspondent pas"),
            RegisterError::PasswordTooShort => {
                write!(f, "Le mot de passe doit contenir au moins 8 caractères")
            }
            RegisterError::InvalidCoachName(e) => write!(f, "{}", e),
            RegisterError::InvalidEmail(e) => write!(f, "{}", e),
            RegisterError::CoachNameAlreadyTaken => write!(f, "Ce nom de coach est déjà utilisé"),
            RegisterError::EmailAlreadyTaken => write!(f, "Cette adresse email est déjà utilisée"),
            RegisterError::PasswordHashError => {
                write!(f, "Erreur lors du chiffrement du mot de passe")
            }
            RegisterError::Database(msg) => write!(f, "Erreur interne : {}", msg),
        }
    }
}

impl From<RepositoryError> for RegisterError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::CoachNameAlreadyTaken => RegisterError::CoachNameAlreadyTaken,
            RepositoryError::EmailAlreadyTaken => RegisterError::EmailAlreadyTaken,
            RepositoryError::Database(msg) => RegisterError::Database(msg),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RegisterCommand,
    repo: &dyn IUserRepository,
    bus: &EventBus,
) -> Result<(), Vec<RegisterError>> {
    let mut errors: Vec<RegisterError> = Vec::new();

    if cmd.password != cmd.password_confirm {
        errors.push(RegisterError::PasswordMismatch);
    }
    if cmd.password.expose().len() < 8 {
        errors.push(RegisterError::PasswordTooShort);
    }

    let coach_name = match CoachName::try_new(&cmd.coach_name) {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(RegisterError::InvalidCoachName(e.into()));
            None
        }
    };
    let email = match Email::try_new(cmd.email.expose()) {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(RegisterError::InvalidEmail(e.into()));
            None
        }
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    let password = cmd.password.expose().clone();
    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|_| vec![RegisterError::PasswordHashError])?
    .map_err(|_| vec![RegisterError::PasswordHashError])?;

    let user = User::new(
        UserId::new(),
        coach_name.unwrap(),
        None,
        email.unwrap(),
        password_hash,
    );

    repo.create(&user)
        .await
        .map_err(|e| vec![RegisterError::from(e)])?;

    let event = AccountCreated {
        event_id: EventId::new(),
        user_id: user.id.clone(),
        user_name: user.coach_name.clone(),
        email: user.email.clone(),
    };
    emettre(bus, event.to_enveloppe());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::domain::user::User;
    use crate::app::auth::ports::RepositoryError;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;

    struct FakeUserRepository {
        pub fail: bool,
    }

    #[async_trait]
    impl IUserRepository for FakeUserRepository {
        async fn create(&self, _: &User) -> Result<(), RepositoryError> {
            if self.fail {
                Err(RepositoryError::Database("db error".into()))
            } else {
                Ok(())
            }
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<User>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_legacy_id(&self, _: i32) -> Result<Option<User>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_coach_name(&self, _: &str) -> Result<Option<User>, RepositoryError> {
            Ok(None)
        }
        async fn update_password_hash(&self, _: &str, _: &str) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    /// Cf. `perform_login` : e-mail compris, l'adresse est une donnée
    /// personnelle qui n'a rien à faire dans un journal de diagnostic.
    #[test]
    fn le_debug_ne_laisse_fuir_ni_mot_de_passe_ni_email() {
        let rendu = format!(
            "{:?}",
            RegisterCommand {
                coach_name: "Bagouze".into(),
                email: "adresse-temoin@example.com".into(),
                password: "hunter2-le-mot-de-passe".into(),
                password_confirm: "hunter2-le-mot-de-passe".into(),
            }
        );

        assert!(!rendu.contains("hunter2"), "mot de passe fuité : {rendu}");
        assert!(!rendu.contains("adresse-temoin"), "e-mail fuité : {rendu}");
        assert!(rendu.contains("Bagouze"), "le diagnostic reste lisible");
    }

    fn valid_command() -> RegisterCommand {
        RegisterCommand {
            coach_name: "Bagouze".into(),
            email: "coach@example.com".into(),
            password: "password123".into(),
            password_confirm: "password123".into(),
        }
    }

    #[tokio::test]
    async fn user_registered_event_is_published_on_success() {
        let repo = FakeUserRepository { fail: false };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        execute(valid_command(), &repo, &bus).await.unwrap();

        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type.as_str(), "AccountCreated");
    }

    #[tokio::test]
    async fn no_event_is_published_when_repo_fails() {
        let repo = FakeUserRepository { fail: true };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(valid_command(), &repo, &bus).await;

        assert!(result.is_err());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn validation_errors_are_returned_without_publishing() {
        let repo = FakeUserRepository { fail: false };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let cmd = RegisterCommand {
            coach_name: "Bagouze".into(),
            email: "coach@example.com".into(),
            password: "court".into(),
            password_confirm: "different".into(),
        };

        let errors = execute(cmd, &repo, &bus).await.unwrap_err();

        assert!(errors.len() >= 2);
        assert!(rx.try_recv().is_err());
    }
}
