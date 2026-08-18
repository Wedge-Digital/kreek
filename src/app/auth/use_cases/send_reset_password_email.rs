use crate::app::auth::domain::reset_token::Token;
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::auth::routes::path;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::common::services::email::{EmailError, IEmailService};
use askama::Template;
use std::fmt;

#[derive(Template)]
#[template(path = "emails/fr_FR/lost_login.html")]
struct LostLoginEmail {
    coach_name: String,
    reset_url: String,
}

#[derive(Debug)]
pub struct SendResetPasswordEmailCommand {
    pub coach_name: CoachName,
    pub host_domain: String,
}

#[derive(Debug)]
pub enum SendResetPasswordEmailError {
    CoachNameNotFound,
    Database(String),
    EmailSendFailed(String),
}

impl fmt::Display for SendResetPasswordEmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoachNameNotFound => write!(f, "Aucun coach avec ce nom"),
            Self::Database(msg) => write!(f, "Erreur base de données : {}", msg),
            Self::EmailSendFailed(msg) => write!(f, "Impossible d'envoyer l'email : {}", msg),
        }
    }
}

impl std::error::Error for SendResetPasswordEmailError {}

impl From<RepositoryError> for SendResetPasswordEmailError {
    fn from(e: RepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<EmailError> for SendResetPasswordEmailError {
    fn from(e: EmailError) -> Self {
        Self::EmailSendFailed(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: SendResetPasswordEmailCommand,
    user_repo: &dyn IUserRepository,
    token_repo: &dyn IResetTokenRepository,
    email_service: &dyn IEmailService,
) -> Result<(), SendResetPasswordEmailError> {
    let coach_name_str = cmd.coach_name.clone().into_inner();

    let user = user_repo
        .find_by_coach_name(&coach_name_str)
        .await?
        .ok_or(SendResetPasswordEmailError::CoachNameNotFound)?;

    let token = Token::new();

    token_repo.create(&token, &user.coach_name).await?;

    let reset_url = format!(
        "http://{}{}/{}",
        cmd.host_domain,
        path::RESET_PASSWORD_BASE,
        token.to_string()
    );
    let html = LostLoginEmail {
        coach_name: coach_name_str.clone(),
        reset_url,
    }
    .render()
    .map_err(|e| SendResetPasswordEmailError::EmailSendFailed(e.to_string()))?;

    email_service
        .send(
            vec![user.email.value().to_string()],
            "Réinitialisation de ton mot de passe BloodbowlClub".to_string(),
            html,
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{execute, SendResetPasswordEmailCommand, SendResetPasswordEmailError};
    use crate::app::auth::domain::reset_token::{ResetToken, Token};
    use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
    use crate::app::auth::io::repository::tests::fake_user_repository::{
        FakeUserRepository, FindResult,
    };
    use crate::app::auth::ports::RepositoryError;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::common::services::email::ConsoleEmailService;
    use async_trait::async_trait;
    use tokio::sync::Mutex;

    struct FakeTokenRepo {
        created: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl IResetTokenRepository for FakeTokenRepo {
        async fn find_by_token(&self, _: &str) -> Result<Option<ResetToken>, RepositoryError> {
            unimplemented!()
        }
        async fn create(&self, token: &Token, _: &CoachName) -> Result<(), RepositoryError> {
            self.created.lock().await.push(token.to_string());
            Ok(())
        }
        async fn delete_by_token(&self, _: &str) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    fn cmd(coach_name: &str) -> SendResetPasswordEmailCommand {
        SendResetPasswordEmailCommand {
            coach_name: CoachName::try_new(coach_name).unwrap(),
            host_domain: "localhost:8080".into(),
        }
    }

    fn token_repo() -> FakeTokenRepo {
        FakeTokenRepo {
            created: Mutex::new(vec![]),
        }
    }

    #[tokio::test]
    async fn success_persiste_le_token_et_envoie_email() {
        let user_repo = FakeUserRepository {
            find_result: FindResult::Found {
                password_hash: "hash".into(),
            },
        };
        let token_repo = token_repo();

        let result = execute(
            cmd("Bagouze"),
            &user_repo,
            &token_repo,
            &ConsoleEmailService,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(token_repo.created.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn coach_inconnu_ne_cree_pas_de_token() {
        let user_repo = FakeUserRepository {
            find_result: FindResult::NotFound,
        };
        let token_repo = token_repo();

        let result = execute(
            cmd("Inconnu"),
            &user_repo,
            &token_repo,
            &ConsoleEmailService,
        )
        .await;

        assert!(matches!(
            result,
            Err(SendResetPasswordEmailError::CoachNameNotFound)
        ));
        assert!(token_repo.created.lock().await.is_empty());
    }

    #[tokio::test]
    async fn erreur_bdd_remontee_sans_envoyer_email() {
        let user_repo = FakeUserRepository {
            find_result: FindResult::DbError("connexion perdue".into()),
        };
        let token_repo = token_repo();

        let result = execute(
            cmd("Bagouze"),
            &user_repo,
            &token_repo,
            &ConsoleEmailService,
        )
        .await;

        assert!(matches!(
            result,
            Err(SendResetPasswordEmailError::Database(_))
        ));
        assert!(token_repo.created.lock().await.is_empty());
    }
}
