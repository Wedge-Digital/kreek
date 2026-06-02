use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::routes::Routes;
use crate::state::AppState;

// ── Widget (barre de recherche + panneau résultats) ──────────────────────────

#[derive(Template)]
#[template(path = "space-coach-search-widget.html")]
pub struct SpaceCoachSearchWidgetTemplate {
    pub search_url: String,
}

impl IntoResponse for SpaceCoachSearchWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_coach_search_widget(
    Path(space_id): Path<String>,
) -> impl IntoResponse {
    SpaceCoachSearchWidgetTemplate {
        search_url: Routes.coach_search(&space_id),
    }
}

// ── Résultats de recherche ────────────────────────────────────────────────────

pub struct CoachResultItem {
    pub id:         String,
    pub coach_name: String,
    pub initials:   String,
}

#[derive(Template)]
#[template(path = "space-coach-search-results.html")]
pub struct CoachSearchResultsTemplate {
    pub coaches: Vec<CoachResultItem>,
}

impl IntoResponse for CoachSearchResultsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q:        String,
    #[serde(default)]
    pub excluded: String,
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;
    use crate::app::auth::auth_backend::AuthBackend;
    use crate::app::auth::context::AuthContext;
    use crate::app::auth::io::repository::tests::fake_reset_token_repository::FakeResetTokenRepository;
    use crate::app::auth::io::repository::tests::fake_user_repository::{FakeUserRepository, FindResult};
    use crate::app::competitions::context::CompetitionsContext;
    use crate::app::competitions::io::repository::tests::fake_competition_repository::FakeCompetitionRepository;
    use crate::app::competitions::io::repository::tests::fake_season_repository::FakeSeasonRepository;
    use crate::app::news::context::NewsContext;
    use crate::app::news::domain::article::Article;
    use crate::app::news::domain::article_repository_port::{ArticleRepositoryError, IArticleRepository};
    use crate::app::news::domain::comment::Comment;
    use crate::app::news::domain::comment_repository_port::{CommentRepositoryError, ICommentRepository};
    use crate::app::shared_kernel::authorization::SpaceProfile;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{ArticleId, CoachId, SpaceId};
    use crate::app::shared_kernel::email::Email;
    use crate::app::spaces::context::SpacesContext;
    use crate::app::spaces::domain::space::Space;
    use crate::app::spaces::domain::space_repository_port::space_repository_port::{ISpaceRepository, SpaceRepositoryError, SpaceSummary};
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::{ISpaceUserCacheRepository, SpaceUserCacheRepositoryError};
    use crate::app::spaces::domain::user::User as SpaceUser;
    use crate::app::spaces::io::web::coach_search::{search_coaches, get_coach_search_widget};
    use crate::app::spaces::routes::path;
    use crate::lib::services::email::fakes::console_email_service::ConsoleEmailService;
    use crate::lib::services::event_bus::event_bus::new_bus;
    use crate::state::AppState;
    use super::initials;

    // ── Helpers ────────────────────────────────────────────────────────────

    fn make_user(name: &str) -> SpaceUser {
        SpaceUser::new(
            CoachId::new(),
            CoachName::try_new(name).unwrap(),
            None,
            Email::try_new("test@test.com").unwrap(),
        )
    }

    struct FakeCache { users: Vec<SpaceUser> }

    #[async_trait]
    impl ISpaceUserCacheRepository for FakeCache {
        async fn add_user(&self, _: &SpaceUser) -> Result<(), SpaceUserCacheRepositoryError> { Ok(()) }
        async fn find_user_by_id(&self, _: &CoachId) -> Result<SpaceUser, SpaceUserCacheRepositoryError> { Err(SpaceUserCacheRepositoryError::UserNotFoundInCache) }
        async fn find_all_users(&self) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> { Ok(vec![]) }
        async fn list_members_for_space(&self, _: &SpaceId) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(self.users.clone())
        }
    }

    struct FakeSpaceRepo;
    #[async_trait]
    impl ISpaceRepository for FakeSpaceRepo {
        async fn save(&self, _: &Space) -> Result<(), SpaceRepositoryError> { Ok(()) }
        async fn add_member(&self, _: &SpaceId, _: &CoachId, _: &SpaceProfile) -> Result<(), SpaceRepositoryError> { Ok(()) }
        async fn join_spaces(&self, _: &[SpaceId], _: &CoachId) -> Result<(), SpaceRepositoryError> { Ok(()) }
        async fn find_by_id(&self, _: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> { Ok(None) }
        async fn find_by_coach_id(&self, _: &CoachId) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> { Ok(vec![]) }
        async fn find_member_profile(&self, _: &CoachId, _: &SpaceId) -> Result<Option<SpaceProfile>, SpaceRepositoryError> { Ok(None) }
        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> { Ok(vec![]) }
    }

    struct FakeArticleRepo;
    #[async_trait]
    impl IArticleRepository for FakeArticleRepo {
        async fn save(&self, _: &Article) -> Result<(), ArticleRepositoryError> { Ok(()) }
        async fn find_by_space(&self, _: &SpaceId, _: i64, _: i64) -> Result<(Vec<Article>, i64), ArticleRepositoryError> { Ok((vec![], 0)) }
        async fn find_by_id(&self, _: &ArticleId) -> Result<Option<Article>, ArticleRepositoryError> { Ok(None) }
    }

    struct FakeCommentRepo;
    #[async_trait]
    impl ICommentRepository for FakeCommentRepo {
        async fn save(&self, _: &Comment) -> Result<(), CommentRepositoryError> { Ok(()) }
        async fn find_by_article(&self, _: &ArticleId) -> Result<Vec<Comment>, CommentRepositoryError> { Ok(vec![]) }
    }

    struct FakeTeamDraftRepository;
    #[async_trait]
    impl crate::app::team_creation::ports::ITeamDraftRepository for FakeTeamDraftRepository {
        async fn save(&self, _: &crate::app::team_creation::domain::team_draft::DraftTeam, _: &str)
            -> Result<(), crate::app::team_creation::ports::RepositoryError> { Ok(()) }
        async fn find_by_id(&self, _: &crate::app::shared_kernel::team::TeamId)
            -> Result<Option<crate::app::team_creation::domain::team_draft::DraftTeam>, crate::app::team_creation::ports::RepositoryError> { Ok(None) }
        async fn find_by_coach_and_space(&self, _: &str, _: &str)
            -> Result<Vec<crate::app::team_creation::domain::team_draft::DraftTeam>, crate::app::team_creation::ports::RepositoryError> { Ok(vec![]) }
    }

    struct FakeTeamRosterRepository;
    #[async_trait]
    impl crate::app::team_creation::ports::ITeamRosterRepository for FakeTeamRosterRepository {
        async fn save(&self, _: &crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam, _: &str)
            -> Result<(), crate::app::team_creation::ports::RepositoryError> { Ok(()) }
        async fn find_by_id(&self, _: &crate::app::shared_kernel::team::TeamId)
            -> Result<Option<crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam>, crate::app::team_creation::ports::RepositoryError> { Ok(None) }
        async fn mark_submitted(&self, _: &crate::app::shared_kernel::team::TeamId)
            -> Result<(), crate::app::team_creation::ports::RepositoryError> { Ok(()) }
        async fn find_submitted_ids_for_space(&self, _: &str)
            -> Result<Vec<String>, crate::app::team_creation::ports::RepositoryError> { Ok(vec![]) }
    }

    fn build_router(users: Vec<SpaceUser>) -> Router {
        use axum_login::AuthManagerLayerBuilder;
        use tower_sessions::{MemoryStore, SessionManagerLayer};

        let user_repo   = Arc::new(FakeUserRepository { find_result: FindResult::NotFound });
        let session_layer = SessionManagerLayer::new(MemoryStore::default());
        let auth_layer    = AuthManagerLayerBuilder::new(
            AuthBackend::new(user_repo.clone() as Arc<dyn crate::app::auth::ports::IUserRepository>, false),
            session_layer,
        ).build();

        let bus = new_bus();
        let state = AppState {
            auth: AuthContext {
                user_repository:        user_repo as Arc<dyn crate::app::auth::ports::IUserRepository>,
                reset_token_repository: Arc::new(FakeResetTokenRepository {
                    find_result: crate::app::auth::io::repository::tests::fake_reset_token_repository::FindResult::NotFound,
                }),
                event_bus: bus.clone(),
            },
            spaces: SpacesContext {
                space_repository:      Arc::new(FakeSpaceRepo),
                user_cache_repository: Arc::new(FakeCache { users }),
                event_bus:             bus.clone(),
            },
            competitions: CompetitionsContext {
                competition_repository: Arc::new(FakeCompetitionRepository),
                season_repository:      Arc::new(FakeSeasonRepository),
                event_bus:              bus.clone(),
            },
            news: NewsContext {
                article_repository: Arc::new(FakeArticleRepo),
                comment_repository: Arc::new(FakeCommentRepo),
            },
            references:    crate::app::references::context::ReferencesContext::new(),
            team_creation: crate::app::team_creation::context::TeamCreationContext {
                team_repository:   Arc::new(FakeTeamDraftRepository),
                roster_repository: Arc::new(FakeTeamRosterRepository),
                event_bus:         bus.clone(),
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
            event_bus:     bus.clone(),
            app_event_bus: new_bus(),
        };

        Router::new()
            .route(path::SPACE_COACH_SEARCH, get(search_coaches))
            .route(path::SPACE_COACH_SEARCH_WIDGET, get(get_coach_search_widget))
            .with_state(state)
            .layer(auth_layer)
    }

    async fn get_body(app: Router, uri: &str) -> (StatusCode, String) {
        let resp = app
            .oneshot(Request::builder().uri(uri).body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes  = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    // ── initials() ────────────────────────────────────────────────────────

    #[test]
    fn initials_single_word() {
        assert_eq!(initials("Bagouze"), "B");
    }

    #[test]
    fn initials_two_words() {
        assert_eq!(initials("Colonel Castor"), "CC");
    }

    #[test]
    fn initials_more_than_two_words_takes_first_two() {
        assert_eq!(initials("Le Grand Tacticien"), "LG");
    }

    #[test]
    fn initials_empty_string() {
        assert_eq!(initials(""), "");
    }

    #[test]
    fn initials_lowercased_input_is_uppercased() {
        assert_eq!(initials("anne-marie dupont"), "AD");
    }

    // ── search_coaches ────────────────────────────────────────────────────

    #[tokio::test]
    async fn empty_query_returns_all_members() {
        let space_id = SpaceId::new().to_string();
        let users    = vec![make_user("Bagouze"), make_user("LeTacticien")];
        let app      = build_router(users);
        let uri      = format!("/app/{}/coaches/search?q=", space_id);

        let (status, body) = get_body(app, &uri).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Bagouze"));
        assert!(body.contains("LeTacticien"));
    }

    #[tokio::test]
    async fn query_filters_by_name_case_insensitive() {
        let space_id = SpaceId::new().to_string();
        let users    = vec![make_user("Bagouze"), make_user("LeTacticien"), make_user("Colonel Castor")];
        let app      = build_router(users);
        let uri      = format!("/app/{}/coaches/search?q=tacticien", space_id);

        let (status, body) = get_body(app, &uri).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("LeTacticien"));
        assert!(!body.contains("Bagouze"));
        assert!(!body.contains("Colonel Castor"));
    }

    #[tokio::test]
    async fn excluded_ids_are_filtered_out() {
        let space_id = SpaceId::new().to_string();
        let user1    = make_user("Bagouze");
        let excluded = user1.id.to_string();
        let users    = vec![user1, make_user("LeTacticien")];
        let app      = build_router(users);
        let uri      = format!("/app/{}/coaches/search?q=&excluded={}", space_id, excluded);

        let (status, body) = get_body(app, &uri).await;

        assert_eq!(status, StatusCode::OK);
        assert!(!body.contains("Bagouze"));
        assert!(body.contains("LeTacticien"));
    }

    #[tokio::test]
    async fn no_results_shows_empty_state() {
        let space_id = SpaceId::new().to_string();
        let app      = build_router(vec![]);
        let uri      = format!("/app/{}/coaches/search?q=", space_id);

        let (status, body) = get_body(app, &uri).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Aucun coach trouvé"));
    }

    #[tokio::test]
    async fn widget_returns_search_url() {
        let space_id = SpaceId::new().to_string();
        let app      = build_router(vec![]);
        let uri      = format!("/app/{}/coaches/search-widget", space_id);

        let (status, body) = get_body(app, &uri).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(&format!("/app/{}/coaches/search", space_id)));
    }
}

pub async fn search_coaches(
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let excluded: std::collections::HashSet<String> = params.excluded
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let query = params.q.trim().to_lowercase();

    let all_members = state.spaces
        .user_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let coaches = all_members
        .into_iter()
        .filter_map(|u| {
            let id   = u.id.to_string();
            if excluded.contains(&id) { return None; }
            let name = u.name.into_inner();
            if !query.is_empty() && !name.to_lowercase().contains(&query) { return None; }
            let ini  = initials(&name);
            Some(CoachResultItem { id, coach_name: name, initials: ini })
        })
        .collect();

    CoachSearchResultsTemplate { coaches }.into_response()
}