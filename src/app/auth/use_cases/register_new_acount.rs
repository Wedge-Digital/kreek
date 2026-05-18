use super::super::domain::error::AuthDomainError;
use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::app::shared_kernel::user::User;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::IEventPublisher;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use crate::app::shared_kernel::coach_name::CoachName;
use std::fmt;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use crate::app::auth::domain::domain_event::AuthDomainEvent::AccountCreated;
use crate::app::shared_kernel::email::Email;

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
    bus: Arc<Mutex<dyn IEventPublisher>>,
) -> Result<(), Vec<RegisterError>> {
    let mut errors: Vec<RegisterError> = Vec::new();

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

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(cmd.password.as_bytes(), &salt)
        .map_err(|_| vec![RegisterError::PasswordHashError])?
        .to_string();

    let user = User::new(
        UserId::new(),
        coach_name.unwrap(),
        None,
        email.unwrap(),
        password_hash,
    );

    repo.create(&user).await.map_err(|e| vec![RegisterError::from(e)])?;

    let event = AccountCreated {
        event_id:   EventId::new(),
        user_id:    user.id.clone(),
        user_name:  user.coach_name.clone(),
        email:      user.email.clone(),
    };

    bus.lock().unwrap().publish(event.to_enveloppe());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use async_trait::async_trait;
    use crate::app::auth::ports::RepositoryError;
    use crate::app::shared_kernel::user::User;

    struct FakeUserRepository { pub fail: bool }

    #[async_trait]
    impl IUserRepository for FakeUserRepository {
        async fn create(&self, _: &User) -> Result<(), RepositoryError> {
            if self.fail { Err(RepositoryError::Database("db error".into())) } else { Ok(()) }
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<User>, RepositoryError> { Ok(None) }
        async fn find_by_coach_name(&self, _: &str) -> Result<Option<User>, RepositoryError> { Ok(None) }
        async fn update_password_hash(&self, _: &str, _: &str) -> Result<(), RepositoryError> { Ok(()) }
    }

    struct FakeEventPublisher { pub published: Mutex<Vec<String>> }

    impl IEventPublisher for FakeEventPublisher {
        fn publish(&self, event: EventEnvelope) {
            self.published.lock().unwrap().push(event.event_type);
        }
    }

    fn valid_command() -> RegisterCommand {
        RegisterCommand {
            coach_name:       "Bagouze".into(),
            email:            "coach@example.com".into(),
            password:         "password123".into(),
            password_confirm: "password123".into(),
        }
    }

    #[tokio::test]
    async fn user_registered_event_is_published_on_success() {
        // Given
        let repo      = FakeUserRepository { fail: false };
        let publisher = Arc::new(Mutex::new(FakeEventPublisher { published: Mutex::new(vec![]) }));

        // When
        execute(valid_command(), &repo, publisher.clone()).await.unwrap();

        // Then
        let publisher = publisher.lock().unwrap();
        let events    = publisher.published.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_str(), "AccountCreated");
    }

    #[tokio::test]
    async fn no_event_is_published_when_repo_fails() {
        // Given
        let repo      = FakeUserRepository { fail: true };
        let publisher = Arc::new(Mutex::new(FakeEventPublisher { published: Mutex::new(vec![]) }));

        // When
        let result = execute(valid_command(), &repo, publisher.clone()).await;

        // Then
        assert!(result.is_err());
        assert!(publisher.lock().unwrap().published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn validation_errors_are_returned_without_publishing() {
        // Given — mot de passe trop court + mismatch
        let repo      = FakeUserRepository { fail: false };
        let publisher = Arc::new(Mutex::new(FakeEventPublisher { published: Mutex::new(vec![]) }));

        let cmd = RegisterCommand {
            coach_name:       "Bagouze".into(),
            email:            "coach@example.com".into(),
            password:         "court".into(),
            password_confirm: "different".into(),
        };

        // When
        let errors = execute(cmd, &repo, publisher.clone()).await.unwrap_err();

        // Then
        assert!(errors.len() >= 2);
        assert!(publisher.lock().unwrap().published.lock().unwrap().is_empty());
    }
}