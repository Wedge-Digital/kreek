use std::fmt;
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use time::OffsetDateTime;
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::app::auth::ports::{IUserRepository, RepositoryError};

const TOKEN_TTL_HOURS: i64 = 1;

pub struct ResetPasswordCommand {
    pub token:            String,
    pub password:         String,
    pub password_confirm: String,
}

#[derive(Debug)]
pub enum ResetPasswordError {
    TokenNotFound,
    TokenExpired,
    PasswordTooShort,
    PasswordMismatch,
    PasswordHashError,
    Database(String),
}

impl fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TokenNotFound    => write!(f, "Ce lien de réinitialisation est invalide"),
            Self::TokenExpired     => write!(f, "Ce lien a expiré, veuillez en demander un nouveau"),
            Self::PasswordTooShort => write!(f, "Le mot de passe doit contenir au moins 8 caractères"),
            Self::PasswordMismatch => write!(f, "Les mots de passe ne correspondent pas"),
            Self::PasswordHashError => write!(f, "Erreur lors du chiffrement du mot de passe"),
            Self::Database(msg)    => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

impl std::error::Error for ResetPasswordError {}

impl From<RepositoryError> for ResetPasswordError {
    fn from(e: RepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

pub async fn execute(
    cmd:        ResetPasswordCommand,
    user_repo:  &dyn IUserRepository,
    token_repo: &dyn IResetTokenRepository,
) -> Result<(), ResetPasswordError> {
    let reset_token = token_repo
        .find_by_token(&cmd.token)
        .await?
        .ok_or(ResetPasswordError::TokenNotFound)?;

    let age = OffsetDateTime::now_utc() - reset_token.created_at;
    if age.whole_hours() >= TOKEN_TTL_HOURS {
        return Err(ResetPasswordError::TokenExpired);
    }

    if cmd.password.len() < 8 {
        return Err(ResetPasswordError::PasswordTooShort);
    }
    if cmd.password != cmd.password_confirm {
        return Err(ResetPasswordError::PasswordMismatch);
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(cmd.password.as_bytes(), &salt)
        .map_err(|_| ResetPasswordError::PasswordHashError)?
        .to_string();

    let coach_name_str = reset_token.coach_name.clone().into_inner();
    user_repo.update_password_hash(&coach_name_str, &hash).await?;
    token_repo.delete_by_token(&cmd.token).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use async_trait::async_trait;
    use crate::app::auth::domain::coach_name::CoachName;
    use crate::app::auth::domain::reset_token::{ResetToken, Token};
    use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};
    use crate::app::auth::ports::RepositoryError;
    use super::{ResetPasswordCommand, ResetPasswordError, execute};

    struct FakeTokenRepo {
        token: Option<ResetToken>,
        deleted: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl IResetTokenRepository for FakeTokenRepo {
        async fn find_by_token(&self, _: &str) -> Result<Option<ResetToken>, RepositoryError> {
            Ok(self.token.as_ref().map(|t| ResetToken {
                token:      t.token,
                coach_name: t.coach_name.clone(),
                created_at: t.created_at,
            }))
        }
        async fn create(&self, _: &Token, _: &CoachName) -> Result<(), RepositoryError> { unimplemented!() }
        async fn delete_by_token(&self, token: &str) -> Result<(), RepositoryError> {
            self.deleted.lock().unwrap().push(token.to_string());
            Ok(())
        }
    }

    fn valid_token() -> ResetToken {
        ResetToken {
            token:      Token::new(),
            coach_name: CoachName::try_new("Bagouze").unwrap(),
            created_at: time::OffsetDateTime::now_utc(),
        }
    }

    fn expired_token() -> ResetToken {
        ResetToken {
            token:      Token::new(),
            coach_name: CoachName::try_new("Bagouze").unwrap(),
            created_at: time::OffsetDateTime::now_utc() - time::Duration::hours(2),
        }
    }

    fn token_repo(token: Option<ResetToken>) -> FakeTokenRepo {
        FakeTokenRepo { token, deleted: Mutex::new(vec![]) }
    }

    fn user_repo() -> FakeUserRepository {
        FakeUserRepository { find_result: FindResult::Found { password_hash: "old_hash".into() } }
    }

    fn cmd(password: &str, confirm: &str) -> ResetPasswordCommand {
        ResetPasswordCommand {
            token: "some-token".into(),
            password: password.into(),
            password_confirm: confirm.into(),
        }
    }

    #[tokio::test]
    async fn success_met_a_jour_le_mot_de_passe_et_supprime_le_token() {
        let token_repo = token_repo(Some(valid_token()));

        let result = execute(cmd("nouveaumdp", "nouveaumdp"), &user_repo(), &token_repo).await;

        assert!(result.is_ok());
        assert_eq!(token_repo.deleted.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn token_inexistant_retourne_erreur() {
        let result = execute(cmd("nouveaumdp", "nouveaumdp"), &user_repo(), &token_repo(None)).await;
        assert!(matches!(result, Err(ResetPasswordError::TokenNotFound)));
    }

    #[tokio::test]
    async fn token_expire_retourne_erreur() {
        let result = execute(cmd("nouveaumdp", "nouveaumdp"), &user_repo(), &token_repo(Some(expired_token()))).await;
        assert!(matches!(result, Err(ResetPasswordError::TokenExpired)));
    }

    #[tokio::test]
    async fn mot_de_passe_trop_court_retourne_erreur() {
        let result = execute(cmd("court", "court"), &user_repo(), &token_repo(Some(valid_token()))).await;
        assert!(matches!(result, Err(ResetPasswordError::PasswordTooShort)));
    }

    #[tokio::test]
    async fn mots_de_passe_differents_retournent_erreur() {
        let result = execute(cmd("nouveaumdp1", "nouveaumdp2"), &user_repo(), &token_repo(Some(valid_token()))).await;
        assert!(matches!(result, Err(ResetPasswordError::PasswordMismatch)));
    }
}