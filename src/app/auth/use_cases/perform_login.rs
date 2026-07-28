use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::auth::domain::domain_event::AuthDomainEvent::UserLoggedIn;
use crate::app::auth::domain::domain_event::USER_LOGGED_IN;
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::app::auth::domain::user::User;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::Deserialize;
use std::fmt;
use time::OffsetDateTime;

#[derive(Debug)]
pub enum LoginError {
    CoachNameNotFound,
    InvalidPassword,
    Database(String),
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::CoachNameNotFound => write!(f, "Coach name not found"),
            LoginError::InvalidPassword => write!(f, "Invalid password"),
            LoginError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl From<RepositoryError> for LoginError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::CoachNameAlreadyTaken | RepositoryError::EmailAlreadyTaken => {
                LoginError::Database("état inattendu".into())
            }
            RepositoryError::Database(msg) => LoginError::Database(msg),
        }
    }
}

#[derive(Deserialize)]
pub struct PerformLoginCommand {
    pub coach_name: String,
    pub password: String,
}

pub async fn execute(
    cmd: PerformLoginCommand,
    repo: &dyn IUserRepository,
    bus: &EventBus,
) -> Result<User, LoginError> {
    let user = repo
        .find_by_coach_name(&cmd.coach_name)
        .await
        .map_err(LoginError::from)?
        .ok_or(LoginError::CoachNameNotFound)?;

    let password_hash = user.password_hash.clone();
    let password = cmd.password.clone();
    let verified = tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|_| LoginError::Database("hash corrompu".into()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| LoginError::InvalidPassword)
    })
    .await
    .map_err(|_| LoginError::Database("spawn_blocking échoué".into()))?;

    verified?;

    let user_id = user.id;
    let event_payload = UserLoggedIn {
        event_id: EventId::new(),
        user_id: user_id.clone(),
    };

    let _ = bus.send(EventEnvelope {
        event_id: UserId::new().to_string(),
        emitter: user_id.to_string(),
        event_type: USER_LOGGED_IN.into(),
        tags: serde_json::json!({ "user_id": user_id }),
        payload: serde_json::json!(event_payload),
        occurred_at: OffsetDateTime::now_utc(),
    });

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::{execute, LoginError, PerformLoginCommand, USER_LOGGED_IN};
    use crate::app::auth::io::repository::tests::fake_user_repository::{
        FakeUserRepository, FindResult,
    };
    use crate::common::services::event_bus::event_bus::new_bus;
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn cmd(coach_name: &str, password: &str) -> PerformLoginCommand {
        PerformLoginCommand {
            coach_name: coach_name.into(),
            password: password.into(),
        }
    }

    #[tokio::test]
    async fn success_publishes_user_logged_in_event() {
        let repo = FakeUserRepository {
            find_result: FindResult::Found {
                password_hash: hash_password("secret"),
            },
        };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(cmd("Bagouze", "secret"), &repo, &bus).await;

        assert!(result.is_ok());
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, USER_LOGGED_IN);
    }

    #[tokio::test]
    async fn coach_name_not_found() {
        let repo = FakeUserRepository {
            find_result: FindResult::NotFound,
        };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(cmd("Unknown", "secret"), &repo, &bus).await;

        assert!(matches!(result, Err(LoginError::CoachNameNotFound)));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn invalid_password() {
        let repo = FakeUserRepository {
            find_result: FindResult::Found {
                password_hash: hash_password("correct"),
            },
        };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(cmd("Bagouze", "wrong"), &repo, &bus).await;

        assert!(matches!(result, Err(LoginError::InvalidPassword)));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn corrupted_hash() {
        let repo = FakeUserRepository {
            find_result: FindResult::Found {
                password_hash: "not_a_valid_argon2_hash".into(),
            },
        };
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(cmd("Bagouze", "secret"), &repo, &bus).await;

        assert!(matches!(result, Err(LoginError::Database(_))));
        assert!(rx.try_recv().is_err());
    }
}
