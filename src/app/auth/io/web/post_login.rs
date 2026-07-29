use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::context::AuthContext;
use crate::app::auth::io::web::get_login::LoginTemplate;
use crate::app::auth::routes::path;
use crate::app::auth::use_cases::perform_login;
use crate::app::auth::use_cases::perform_login::{LoginError, PerformLoginCommand};
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Form;

pub async fn login_submit(
    mut auth_session: AuthSession,
    State(ctx): State<AuthContext>,
    Form(payload): Form<PerformLoginCommand>,
) -> impl IntoResponse {
    match perform_login::execute(payload, ctx.user_repository.as_ref(), &ctx.event_bus).await {
        Ok(user) => {
            if auth_session.login(&user).await.is_err() {
                return LoginTemplate {
                    login_error: Some("Erreur de session, réessaie.".into()),
                    ..Default::default()
                }
                .into_response();
            }
            let mut response = Response::new(axum::body::Body::empty());
            response.headers_mut().insert(
                header::HeaderName::from_static("hx-redirect"),
                header::HeaderValue::from_static(path::LOGIN_SUCCESS),
            );
            response
        }
        Err(LoginError::CoachNameNotFound) | Err(LoginError::InvalidPassword) => LoginTemplate {
            login_error: Some("Nom de coach ou mot de passe incorrect.".into()),
            ..Default::default()
        }
        .into_response(),
        Err(LoginError::Database(_)) => LoginTemplate {
            login_error: Some("Erreur interne, réessaie plus tard.".into()),
            ..Default::default()
        }
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::auth::auth_backend::AuthBackend;
    use crate::app::auth::context::AuthContext;
    use crate::app::auth::io::repository::tests::fake_reset_token_repository::FakeResetTokenRepository;
    use crate::app::auth::io::repository::tests::fake_user_repository::{
        FakeUserRepository, FindResult,
    };
    use crate::app::auth::io::web::post_login::login_submit;
    use crate::app::auth::routes::path;
    use crate::common::services::email::fakes::console_email_service::ConsoleEmailService;
    use crate::common::services::event_bus::event_bus::new_bus;
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};
    use axum::body::to_bytes;
    use axum::routing::post;
    use axum::Router;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn build_app(find_result: FindResult) -> Router {
        use axum_login::AuthManagerLayerBuilder;
        use tower_sessions::{MemoryStore, SessionManagerLayer};

        let mock = Arc::new(FakeUserRepository { find_result });
        let session_layer = SessionManagerLayer::new(MemoryStore::default());
        let auth_layer = AuthManagerLayerBuilder::new(
            AuthBackend::new(mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>),
            session_layer,
        )
        .build();

        let event_bus = new_bus();

        // Le handler ne prend plus que son propre contexte : tester un login ne
        // demande plus de construire les dix BCs de l'application. Les fakes
        // news / competitions / teams qui vivaient ici — et que chaque nouvelle
        // méthode de leurs ports cassait — ont disparu avec `AppState`.
        let ctx = AuthContext {
            user_repository: mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>,
            reset_token_repository: Arc::new(FakeResetTokenRepository {
                find_result: crate::app::auth::io::repository::tests::fake_reset_token_repository::FindResult::NotFound,
            }),
            event_bus: event_bus.clone(),
            email_service: Arc::new(ConsoleEmailService),
            host_domain: "localhost".to_string(),
            authenticated_home: "/app".to_string(),
        };

        Router::new()
            .route(path::LOGIN, post(login_submit))
            .with_state(ctx)
            .layer(auth_layer)
    }

    async fn post_login(app: Router, body: &str) -> axum::response::Response {
        use axum::http::Request;
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(path::LOGIN)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn success_sets_hx_redirect() {
        let response = post_login(
            build_app(FindResult::Found {
                password_hash: hash_password("secret"),
            }),
            "coach_name=Bagouze&password=secret",
        )
        .await;

        assert_eq!(
            response
                .headers()
                .get("hx-redirect")
                .and_then(|v| v.to_str().ok()),
            Some(path::LOGIN_SUCCESS),
        );
    }

    #[tokio::test]
    async fn invalid_password_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::Found {
                password_hash: hash_password("correct"),
            }),
            "coach_name=Bagouze&password=wrong",
        )
        .await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn unknown_coach_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::NotFound),
            "coach_name=Inconnu&password=secret",
        )
        .await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn database_error_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::DbError("connexion refusée".into())),
            "coach_name=Bagouze&password=secret",
        )
        .await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("Erreur interne"));
    }
}
