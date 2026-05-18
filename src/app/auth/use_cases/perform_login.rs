use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::user::User;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use std::fmt;
use std::sync::{Arc, Mutex};
use serde::Deserialize;
use time::OffsetDateTime;
use crate::app::auth::domain::domain_event::AuthDomainEvent::UserLoggedIn;
use crate::app::auth::domain::domain_event::USER_LOGGED_IN;
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::IEventPublisher;

#[derive(Debug)]
pub enum LoginError {
    CoachNameNotFound,
    InvalidPassword,
    Database(String),
}

#[derive(Deserialize, Debug)]
pub struct PerformLoginCommand {
    pub coach_name: String,
    pub password:   String,
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::CoachNameNotFound => write!(f, "Aucun compte trouvé pour ce nom de coach"),
            LoginError::InvalidPassword   => write!(f, "Mot de passe incorrect"),
            LoginError::Database(msg)     => write!(f, "Erreur interne : {}", msg),
        }
    }
}

impl From<RepositoryError> for LoginError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::CoachNameAlreadyTaken
            | RepositoryError::EmailAlreadyTaken => LoginError::Database("état inattendu".into()),
            RepositoryError::Database(msg)       => LoginError::Database(msg),
        }
    }
}

pub async fn execute(
    cmd: PerformLoginCommand,
    repo: &dyn IUserRepository,
    bus: Arc<Mutex<dyn IEventPublisher>>,
) -> Result<User, LoginError> {
    let user = repo
        .find_by_coach_name(&cmd.coach_name)
        .await
        .map_err(LoginError::from)?
        .ok_or(LoginError::CoachNameNotFound)?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| LoginError::Database("hash corrompu".into()))?;

    Argon2::default()
        .verify_password(cmd.password.as_bytes(), &parsed_hash)
        .map_err(|_| LoginError::InvalidPassword)?;


    let user_id   = user.id;
    let event_payload = UserLoggedIn {
        event_id:   EventId::new(),
        user_id:    user_id.clone(),
    };

    bus.lock().unwrap().publish(EventEnvelope {
        event_id:   UserId::new().to_string(),
        emitter:    user_id.to_string().clone(),
        event_type: USER_LOGGED_IN.into(),
        tags:       serde_json::json!({ "user_id": user_id }),
        payload:    serde_json::json!(event_payload),
        occurred_at: OffsetDateTime::now_utc(),
    });

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::{execute, LoginError, PerformLoginCommand, USER_LOGGED_IN};
    use std::sync::{Arc, Mutex};
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};
    use crate::lib::event_envelope::EventEnvelope;
    use crate::lib::services::event_bus::event_bus::IEventPublisher;

    struct FakeEventPublisher { published: Mutex<Vec<String>> }

    impl IEventPublisher for FakeEventPublisher {
        fn publish(&self, event: EventEnvelope) {
            self.published.lock().unwrap().push(event.event_type);
        }
    }

    fn fake_bus() -> Arc<Mutex<FakeEventPublisher>> {
        Arc::new(Mutex::new(FakeEventPublisher { published: Mutex::new(vec![]) }))
    }

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }

    fn cmd(coach_name: &str, password: &str) -> PerformLoginCommand {
        PerformLoginCommand { coach_name: coach_name.into(), password: password.into() }
    }

    #[tokio::test]
    async fn success_publishes_user_logged_in_event() {
        let repo = FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("secret") } };
        let bus  = fake_bus();

        let result = execute(cmd("Bagouze", "secret"), &repo, bus.clone()).await;

        assert!(result.is_ok());
        assert_eq!(*bus.lock().unwrap().published.lock().unwrap(), vec![USER_LOGGED_IN]);
    }

    #[tokio::test]
    async fn coach_name_not_found() {
        let repo = FakeUserRepository { find_result: FindResult::NotFound };
        let bus  = fake_bus();

        let result = execute(cmd("Unknown", "secret"), &repo, bus.clone()).await;

        assert!(matches!(result, Err(LoginError::CoachNameNotFound)));
        assert!(bus.lock().unwrap().published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_password() {
        let repo = FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("correct") } };
        let bus  = fake_bus();

        let result = execute(cmd("Bagouze", "wrong"), &repo, bus.clone()).await;

        assert!(matches!(result, Err(LoginError::InvalidPassword)));
        assert!(bus.lock().unwrap().published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn corrupted_hash() {
        let repo = FakeUserRepository {
            find_result: FindResult::Found { password_hash: "not_a_valid_argon2_hash".into() },
        };
        let bus = fake_bus();

        let result = execute(cmd("Bagouze", "secret"), &repo, bus.clone()).await;

        assert!(matches!(result, Err(LoginError::Database(_))));
        assert!(bus.lock().unwrap().published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn database_error_propagates() {
        let repo = FakeUserRepository { find_result: FindResult::DbError("connexion refusée".into()) };
        let bus  = fake_bus();

        let result = execute(cmd("Bagouze", "secret"), &repo, bus.clone()).await;

        assert!(matches!(result, Err(LoginError::Database(_))));
        assert!(bus.lock().unwrap().published.lock().unwrap().is_empty());
    }
}
