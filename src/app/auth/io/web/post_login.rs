use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::io::web::get_login::LoginTemplate;
use crate::app::auth::routes::path;
use crate::app::auth::use_cases::perform_login;
use crate::app::auth::use_cases::perform_login::{LoginError, PerformLoginCommand};
use crate::state::AppState;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Form;

pub async fn login_submit(
    mut auth_session: AuthSession,
    State(state): State<AppState>,
    Form(payload): Form<PerformLoginCommand>,
) -> impl IntoResponse {
    match perform_login::execute(payload, state.user_repository.as_ref(), state.domain_event_bus.as_ref()).await {
        Ok(user) => {
            if auth_session.login(&user).await.is_err() {
                return LoginTemplate {
                    login_error: Some("Erreur de session, réessaie.".into()),
                    ..Default::default()
                }.into_response();
            }
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
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;
    use crate::app::auth::auth_backend::AuthBackend;
    use crate::app::auth::io::repository::tests::fake_reset_token_repository::FakeResetTokenRepository;
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};
    use crate::app::auth::routes::path;
    use crate::app::shared_kernel::authorization::SpaceAuthorization;
    use crate::app::shared_kernel::common_types::{CoachId, SpaceId};
    use crate::lib::services::email::fakes::console_email_service::ConsoleEmailService;
    use crate::app::spaces::domain::ports::{ISpaceRepository, SpaceRepositoryError};
    use crate::lib::services::event_bus::event_bus::EventBus;
    use crate::state::AppState;
    use super::login_submit;

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }

    fn build_app(find_result: FindResult) -> Router {
        use axum_login::AuthManagerLayerBuilder;
        use tower_sessions::{MemoryStore, SessionManagerLayer};

        let mock = Arc::new(FakeUserRepository { find_result });
        let session_layer = SessionManagerLayer::new(MemoryStore::default());
        let auth_layer = AuthManagerLayerBuilder::new(
            AuthBackend::new(mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>, false),
            session_layer,
        ).build();

        let state = AppState {
            user_repository:        mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>,
            email_service:          Arc::new(ConsoleEmailService),
            reset_token_repository: Arc::new(FakeResetTokenRepository {
                find_result: crate::app::auth::io::repository::tests::fake_reset_token_repository::FindResult::NotFound,
            }),
            space_repository:       Arc::new(FakeSpaceRepository),
            host_domain:            "localhost:8080".into(),
            bypass_auth:            false,
            domain_event_bus:       Arc::new(EventBus::new()),
        };

        Router::new()
            .route(path::LOGIN, post(login_submit))
            .with_state(state)
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

    struct FakeSpaceRepository;
    #[async_trait::async_trait]
    impl ISpaceRepository for FakeSpaceRepository {
        async fn save(&self, _: &crate::app::spaces::domain::space::Space) -> Result<(), crate::app::spaces::domain::ports::SpaceRepositoryError> { Ok(()) }
        async fn add_member(&self, _: &crate::app::shared_kernel::common_types::SpaceId, _: &crate::app::shared_kernel::common_types::CoachId, _: &crate::app::shared_kernel::authorization::SpaceAuthorization) -> Result<(), crate::app::spaces::domain::ports::SpaceRepositoryError> { Ok(()) }
        async fn find_by_id(&self, _: &crate::app::shared_kernel::common_types::SpaceId) -> Result<Option<crate::app::spaces::domain::space::Space>, crate::app::spaces::domain::ports::SpaceRepositoryError> { Ok(None) }
        async fn find_by_coach_id(&self, _: &crate::app::shared_kernel::common_types::CoachId) -> Result<Vec<crate::app::spaces::domain::ports::SpaceSummary>, crate::app::spaces::domain::ports::SpaceRepositoryError> { Ok(vec![]) }

        async fn find_member_profile(&self, coach_id: &CoachId, space_id: &SpaceId) -> Result<Option<SpaceAuthorization>, SpaceRepositoryError> {
            Ok(Some(SpaceAuthorization::SimpleUser))
        }

        async fn find_all(&self) -> Result<Vec<crate::app::spaces::domain::ports::SpaceSummary>, crate::app::spaces::domain::ports::SpaceRepositoryError> { Ok(vec![]) }
    }

    #[tokio::test]
    async fn success_sets_hx_redirect() {
        let response = post_login(
            build_app(FindResult::Found { password_hash: hash_password("secret") }),
            "coach_name=Bagouze&password=secret",
        ).await;

        assert_eq!(
            response.headers().get("hx-redirect").and_then(|v| v.to_str().ok()),
            Some(path::LOGIN_SUCCESS),
        );
    }

    #[tokio::test]
    async fn invalid_password_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::Found { password_hash: hash_password("correct") }),
            "coach_name=Bagouze&password=wrong",
        ).await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn unknown_coach_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::NotFound),
            "coach_name=Inconnu&password=secret",
        ).await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("incorrect"));
    }

    #[tokio::test]
    async fn database_error_returns_error_fragment() {
        let response = post_login(
            build_app(FindResult::DbError("connexion refusée".into())),
            "coach_name=Bagouze&password=secret",
        ).await;

        assert!(response.headers().get("hx-redirect").is_none());
        assert!(body_string(response).await.contains("Erreur interne"));
    }
}