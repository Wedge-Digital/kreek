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
    match perform_login::execute(
        payload,
        state.auth.user_repository.as_ref(),
        &state.event_bus,
    )
    .await
    {
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
    use crate::app::competitions::context::CompetitionsContext;
    use crate::app::competitions::io::repository::tests::fake_competition_repository::FakeCompetitionRepository;
    use crate::app::competitions::io::repository::tests::fake_season_repository::FakeSeasonRepository;
    use crate::app::news::context::NewsContext;
    use crate::app::news::domain::article::Article;
    use crate::app::news::domain::article_repository_port::{
        ArticleRepositoryError, IArticleRepository,
    };
    use crate::app::news::domain::comment::Comment;
    use crate::app::news::domain::comment_repository_port::{
        CommentRepositoryError, ICommentRepository,
    };
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::common_types::{ArticleId, CoachId, SpaceId};
    use crate::app::spaces::context::SpacesContext;
    use crate::app::spaces::domain::space::Space;
    use crate::app::spaces::domain::space_repository_port::space_repository_port::{
        ISpaceRepository, SpaceRepositoryError, SpaceSummary,
    };
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::{
        ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
    };
    use crate::app::spaces::domain::user::User as SpaceUser;
    use crate::common::services::email::fakes::console_email_service::ConsoleEmailService;
    use crate::common::services::event_bus::event_bus::new_bus;
    use crate::state::AppState;
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
            AuthBackend::new(
                mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>,
                false,
            ),
            session_layer,
        )
        .build();

        let event_bus = new_bus();
        let app_event_bus = new_bus();

        let state = AppState {
            auth: AuthContext {
                user_repository:        mock.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>,
                reset_token_repository: Arc::new(FakeResetTokenRepository {
                    find_result: crate::app::auth::io::repository::tests::fake_reset_token_repository::FindResult::NotFound,
                }),
                event_bus: event_bus.clone(),
            },
            spaces: SpacesContext {
                space_repository:      Arc::new(FakeSpaceRepository),
                user_cache_repository: Arc::new(FakeUserCacheRepository),
                event_bus:             event_bus.clone(),
            },
            competitions: CompetitionsContext {
                competition_repository: Arc::new(FakeCompetitionRepository),
                season_repository:      Arc::new(FakeSeasonRepository),
                event_bus:              event_bus.clone(),
            },
            news: NewsContext {
                article_repository: Arc::new(FakeNewsRepository),
                comment_repository: Arc::new(FakeCommentRepository),
            },
            references:    crate::app::references::context::ReferencesContext::new(),
            team_creation: crate::app::team_creation::context::TeamCreationContext {
                team_repository:   Arc::new(FakeTeamDraftRepository),
                roster_repository: Arc::new(FakeTeamRosterRepository),
                event_bus:         event_bus.clone(),
            },
            teams: {
                struct FakeTeamRepo;
                #[async_trait::async_trait]
                impl crate::app::teams::ports::ITeamRepository for FakeTeamRepo {
                    async fn append(&self, _: &str, _: &crate::app::teams::domain::team::TeamDomainEvent, _: u64)
                        -> Result<u64, crate::app::teams::ports::RepositoryError> { Ok(1) }
                    async fn find_by_id(&self, _: &str)
                        -> Result<Option<crate::app::teams::domain::team::Team>, crate::app::teams::ports::RepositoryError> { Ok(None) }
                }
                crate::app::teams::context::TeamsContext {
                    team_repository: Arc::new(FakeTeamRepo),
                }
            },
            email_service: Arc::new(ConsoleEmailService),
            host_domain:   "localhost:8080".into(),
            bypass_auth:   false,
            event_bus:     event_bus.clone(),
            app_event_bus: app_event_bus.clone(),
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

    struct FakeNewsRepository;
    #[async_trait::async_trait]
    impl IArticleRepository for FakeNewsRepository {
        async fn save(&self, _: &Article) -> Result<(), ArticleRepositoryError> {
            Ok(())
        }
        async fn find_by_space(
            &self,
            _: &SpaceId,
            _: i64,
            _: i64,
        ) -> Result<(Vec<Article>, i64), ArticleRepositoryError> {
            Ok((vec![], 0))
        }
        async fn find_by_id(
            &self,
            _: &ArticleId,
        ) -> Result<Option<Article>, ArticleRepositoryError> {
            Ok(None)
        }
    }

    struct FakeCommentRepository;
    #[async_trait::async_trait]
    impl ICommentRepository for FakeCommentRepository {
        async fn save(&self, _: &Comment) -> Result<(), CommentRepositoryError> {
            Ok(())
        }
        async fn find_by_article(
            &self,
            _: &ArticleId,
        ) -> Result<Vec<Comment>, CommentRepositoryError> {
            Ok(vec![])
        }
    }

    struct FakeUserCacheRepository;
    #[async_trait::async_trait]
    impl ISpaceUserCacheRepository for FakeUserCacheRepository {
        async fn add_user(&self, _: &SpaceUser) -> Result<(), SpaceUserCacheRepositoryError> {
            Ok(())
        }
        async fn find_user_by_id(
            &self,
            _: &CoachId,
        ) -> Result<SpaceUser, SpaceUserCacheRepositoryError> {
            Err(SpaceUserCacheRepositoryError::UserNotFoundInCache)
        }
        async fn find_all_users(&self) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
        async fn list_members_for_space(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
    }

    struct FakeSpaceRepository;
    #[async_trait::async_trait]
    impl ISpaceRepository for FakeSpaceRepository {
        async fn save(&self, _: &Space) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }
        async fn add_member(
            &self,
            _: &SpaceId,
            _: &CoachId,
            _: &SpaceProfile,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn join_spaces(
            &self,
            _: &[SpaceId],
            _: &CoachId,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn find_by_id(&self, _: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
            Ok(None)
        }
        async fn find_by_coach_id(
            &self,
            _: &CoachId,
        ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }

        async fn find_member_profile(
            &self,
            _coach_id: &CoachId,
            _space_id: &SpaceId,
        ) -> Result<Option<SpaceProfile>, SpaceRepositoryError> {
            Ok(Some(SpaceProfile::SpaceUser))
        }

        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }
    }

    struct FakeTeamDraftRepository;
    #[async_trait::async_trait]
    impl crate::app::team_creation::ports::ITeamDraftRepository for FakeTeamDraftRepository {
        async fn save(
            &self,
            _: &crate::app::team_creation::domain::team_draft::DraftTeam,
            _: &str,
        ) -> Result<(), crate::app::team_creation::ports::RepositoryError> {
            Ok(())
        }
        async fn find_by_id(
            &self,
            _: &crate::app::shared_kernel::team::TeamId,
        ) -> Result<
            Option<crate::app::team_creation::domain::team_draft::DraftTeam>,
            crate::app::team_creation::ports::RepositoryError,
        > {
            Ok(None)
        }
        async fn find_by_coach_and_space(
            &self,
            _: &str,
            _: &str,
        ) -> Result<
            Vec<crate::app::team_creation::domain::team_draft::DraftTeam>,
            crate::app::team_creation::ports::RepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeTeamRosterRepository;
    #[async_trait::async_trait]
    impl crate::app::team_creation::ports::ITeamRosterRepository for FakeTeamRosterRepository {
        async fn save(
            &self,
            _: &crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam,
            _: &str,
        ) -> Result<(), crate::app::team_creation::ports::RepositoryError> {
            Ok(())
        }
        async fn find_by_id(
            &self,
            _: &crate::app::shared_kernel::team::TeamId,
        ) -> Result<
            Option<crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam>,
            crate::app::team_creation::ports::RepositoryError,
        > {
            Ok(None)
        }
        async fn mark_submitted(
            &self,
            _: &crate::app::shared_kernel::team::TeamId,
        ) -> Result<(), crate::app::team_creation::ports::RepositoryError> {
            Ok(())
        }
        async fn find_submitted_ids_for_space(
            &self,
            _: &str,
        ) -> Result<Vec<String>, crate::app::team_creation::ports::RepositoryError> {
            Ok(vec![])
        }
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
