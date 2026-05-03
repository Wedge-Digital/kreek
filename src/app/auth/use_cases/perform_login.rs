use super::super::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::user::User;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use std::fmt;
use serde::Deserialize;

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

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::{execute, LoginError, PerformLoginCommand};
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn cmd(coach_name: &str, password: &str) -> PerformLoginCommand {
        PerformLoginCommand { coach_name: coach_name.into(), password: password.into() }
    }

    #[tokio::test]
    async fn success() {
        let repo = FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("secret") } };
        let result = execute(cmd("Bagouze", "secret"), &repo).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn coach_name_not_found() {
        let repo = FakeUserRepository { find_result: FindResult::NotFound };
        let result = execute(cmd("Unknown", "secret"), &repo).await;
        assert!(matches!(result, Err(LoginError::CoachNameNotFound)));
    }

    #[tokio::test]
    async fn invalid_password() {
        let repo = FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("correct") } };
        let result = execute(cmd("Bagouze", "wrong"), &repo).await;
        assert!(matches!(result, Err(LoginError::InvalidPassword)));
    }

    #[tokio::test]
    async fn corrupted_hash() {
        let repo = FakeUserRepository {
            find_result: FindResult::Found { password_hash: "not_a_valid_argon2_hash".into() },
        };
        let result = execute(cmd("Bagouze", "secret"), &repo).await;
        assert!(matches!(result, Err(LoginError::Database(_))));
    }

    #[tokio::test]
    async fn database_error_propagates() {
        let repo = FakeUserRepository { find_result: FindResult::DbError("connexion refusée".into()) };
        let result = execute(cmd("Bagouze", "secret"), &repo).await;
        assert!(matches!(result, Err(LoginError::Database(_))));
    }
}
