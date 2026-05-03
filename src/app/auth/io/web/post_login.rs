use crate::app::auth::io::web::get_login::LoginTemplate;
use crate::app::auth::routes::path;
use crate::app::auth::use_cases::perform_login;
use crate::app::auth::use_cases::perform_login::{LoginError, PerformLoginCommand};
use crate::state::AppState;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Form;
use tracing::debug;

pub async fn login_submit(
    State(state): State<AppState>,
    Form(payload): Form<PerformLoginCommand>) ->impl IntoResponse {
    debug!(coach_name = %payload.coach_name, "login form received");

    match perform_login::execute(payload, state.user_repository.as_ref()).await {
        Ok(user) => {
            //connexion réussie → redirect HTMX
            let mut response = Response::new(axum::body::Body::empty());
            response.headers_mut().insert(
                header::HeaderName::from_static("hx-redirect"),
                header::HeaderValue::from_static(path::LOGIN_SUCCESS),
            );
            response
        }
        Err(LoginError::CoachNameNotFound) | Err(LoginError::InvalidPassword) => {
            LoginTemplate {
                login_error: Some("Nom de coach ou mot de passe incorrect.".into()),
                ..Default::default()
            }.into_response()
        }
        Err(LoginError::Database(_)) => {
            LoginTemplate {
                login_error: Some("Erreur interne, réessaie plus tard.".into()),
                ..Default::default()
            }.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::{SaltString, rand_core::OsRng};
    use axum::body::to_bytes;
    use axum::extract::State;
    use axum::Form;
    use axum::response::IntoResponse;
    use crate::state::AppState;
    use super::login_submit;
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};
    use crate::app::auth::io::repository::tests::fake_reset_token_repository::FakeResetTokenRepository;
    use crate::app::auth::routes::path;
    use crate::app::auth::use_cases::perform_login::PerformLoginCommand;
    use crate::lib::services::email::fakes::console_email_service::ConsoleEmailService;

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }

    fn state(mock: FakeUserRepository) -> State<AppState> {
        State(AppState {
            user_repository: Arc::new(mock),
            email_service: Arc::new(ConsoleEmailService),
            reset_token_repository: Arc::new(FakeResetTokenRepository { find_result: crate::app::auth::io::repository::tests::fake_reset_token_repository::FindResult::NotFound }),
            host_domain: "localhost:8080".into(),
        })
    }

    fn form(coach_name: &str, password: &str) -> Form<PerformLoginCommand> {
        Form(PerformLoginCommand { coach_name: coach_name.into(), password: password.into() })
    }

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn success_sets_hx_redirect() {
        let response = login_submit(
            state(FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("secret") } }),
            form("Bagouze", "secret"),
        ).await.into_response();

        assert_eq!(
            response.headers().get("hx-redirect").and_then(|v| v.to_str().ok()),
            Some(path::LOGIN_SUCCESS),
        );
    }

    #[tokio::test]
    async fn invalid_password_returns_error_fragment() {
        let response = login_submit(
            state(FakeUserRepository { find_result: FindResult::Found { password_hash: hash_password("correct") } }),
            form("Bagouze", "wrong"),
        ).await.into_response();

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn unknown_coach_returns_error_fragment() {
        let response = login_submit(
            state(FakeUserRepository { find_result: FindResult::NotFound }),
            form("Inconnu", "secret"),
        ).await.into_response();

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn database_error_returns_error_fragment() {
        let response = login_submit(
            state(FakeUserRepository { find_result: FindResult::DbError("connexion refusée".into()) }),
            form("Bagouze", "secret"),
        ).await.into_response();

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("Erreur interne"));
    }
}
