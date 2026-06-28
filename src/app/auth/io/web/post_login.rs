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
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
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
            competitions: {
                struct FakeGroupRepo;
                #[async_trait::async_trait]
                impl crate::app::competitions::domain::group_repository_port::IGroupRepository for FakeGroupRepo {
                    async fn find_groups(&self, _: &str) -> Result<Vec<crate::app::competitions::domain::group_repository_port::GroupWithTeams>, crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(vec![]) }
                    async fn save_assignments(&self, _: &[(String, String)]) -> Result<(), crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(()) }
                    async fn reset_assignments(&self, _: &str) -> Result<(), crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(()) }
                    async fn assign_team(&self, _: &str, _: &str) -> Result<(), crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(()) }
                    async fn unassign_team(&self, _: &str) -> Result<(), crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(()) }
                    async fn ensure_groups_from_structure(&self, _: &str, _: &[(String, String)]) -> Result<(), crate::app::competitions::domain::group_repository_port::GroupRepositoryError> { Ok(()) }
                }
                struct FakeTeamInfoPort;
                #[async_trait::async_trait]
                impl crate::app::competitions::ports::ITeamInfoPort for FakeTeamInfoPort {
                    async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<crate::app::competitions::ports::TeamInfoDto>, String> { Ok(vec![]) }
                }
                struct FakeMatchDayRepo;
                #[async_trait::async_trait]
                impl crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository for FakeMatchDayRepo {
                    async fn find_by_season(&self, _: &str) -> Result<Vec<crate::app::competitions::domain::match_day::MatchDay>, crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(vec![]) }
                    async fn find_by_id(&self, _: &str) -> Result<Option<crate::app::competitions::domain::match_day::MatchDay>, crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(None) }
                    async fn save_match_day(&self, _: &crate::app::competitions::domain::match_day::MatchDay) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn delete_match_day(&self, _: &str) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn save_pairing(&self, _: &str, _: &crate::app::competitions::domain::match_day::Pairing) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn delete_pairing(&self, _: &str) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn clear_pairings(&self, _: &str) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn clear_all_pairings(&self, _: &str) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                    async fn ensure_match_days_from_structure(&self, _: &str, _: &[(String, String, String, Option<String>, Option<String>)]) -> Result<(), crate::app::competitions::domain::match_day_repository_port::MatchDayRepositoryError> { Ok(()) }
                }
                CompetitionsContext {
                    competition_repository: Arc::new(FakeCompetitionRepository),
                    season_repository:      Arc::new(FakeSeasonRepository),
                    group_repository:       Arc::new(FakeGroupRepo),
                    match_day_repository:   Arc::new(FakeMatchDayRepo),
                    team_info_port:         Arc::new(FakeTeamInfoPort),
                    event_bus:              event_bus.clone(),
                }
            },
            news: NewsContext {
                article_repository: Arc::new(FakeNewsRepository),
                comment_repository: Arc::new(FakeCommentRepository),
            },
            references:    crate::app::references::context::ReferencesContext::new(),
            team_creation: {
                struct FakeCompetitionRules;
                #[async_trait::async_trait]
                impl crate::app::team_creation::ports::ICompetitionCreationRulesPort for FakeCompetitionRules {
                    async fn find_creation_rules_for_season(&self, _: &str) -> Option<crate::app::team_creation::domain::creation_rules::CreationRules> { None }
                }
                crate::app::team_creation::context::TeamCreationContext {
                    team_repository:   Arc::new(FakeTeamDraftRepository),
                    roster_repository: Arc::new(FakeTeamRosterRepository),
                    reference_data:    Arc::new(FakeReferenceData),
                    competition_rules: Arc::new(FakeCompetitionRules),
                    event_bus:         event_bus.clone(),
                }
            },
            teams: {
                struct FakeTeamRepo;
                #[async_trait::async_trait]
                impl crate::app::teams::ports::ITeamRepository for FakeTeamRepo {
                    async fn append(&self, _: &str, _: &crate::app::teams::domain::team::TeamDomainEvent, _: u64)
                        -> Result<u64, crate::app::teams::ports::RepositoryError> { Ok(1) }
                    async fn find_by_id(&self, _: &str)
                        -> Result<Option<crate::app::teams::domain::team::Team>, crate::app::teams::ports::RepositoryError> { Ok(None) }
                    async fn find_by_season_and_status(&self, _: &str, _: &str)
                        -> Result<Vec<crate::app::teams::ports::TeamEnrollmentRow>, crate::app::teams::ports::RepositoryError> { Ok(vec![]) }
                    async fn find_enrolled_for_season(&self, _: &str)
                        -> Result<Vec<crate::app::teams::ports::TeamCardRow>, crate::app::teams::ports::RepositoryError> { Ok(vec![]) }
                }
                struct FakePlayerCountPort;
                #[async_trait::async_trait]
                impl crate::app::teams::ports::IPlayerCountPort for FakePlayerCountPort {
                    async fn count_for_team(&self, _: &str) -> u32 { 0 }
                }
                struct FakeJourneymanTypePort;
                impl crate::app::teams::ports::IJourneymanTypePort for FakeJourneymanTypePort {
                    fn journeyman_type_for_roster(&self, _: &str) -> String { String::new() }
                }
                crate::app::teams::context::TeamsContext {
                    team_repository:       Arc::new(FakeTeamRepo),
                    player_count_port:     Arc::new(FakePlayerCountPort),
                    journeyman_type_port:  Arc::new(FakeJourneymanTypePort),
                }
            },
            players: {
                struct FakePlayerRepo;
                #[async_trait::async_trait]
                impl crate::app::players::ports::IPlayerRepository for FakePlayerRepo {
                    async fn append(&self, _: &crate::app::players::domain::player::PlayerId, _: &crate::app::players::domain::player::TeamId, _: &crate::app::players::domain::events::PlayerDomainEvent, _: i32)
                        -> Result<(), crate::app::players::ports::RepositoryError> { Ok(()) }
                    async fn find_by_id(&self, _: &crate::app::players::domain::player::PlayerId)
                        -> Result<Option<crate::app::players::domain::player::Player>, crate::app::players::ports::RepositoryError> { Ok(None) }
                    async fn find_by_team_id(&self, _: &crate::app::players::domain::player::TeamId)
                        -> Result<Vec<crate::app::players::domain::player::Player>, crate::app::players::ports::RepositoryError> { Ok(vec![]) }
                }
                struct FakePlayerProjectionRepo;
                #[async_trait::async_trait]
                impl crate::app::players::ports::IPlayerProjectionRepository for FakePlayerProjectionRepo {
                    async fn find_by_team_id(&self, _: &crate::app::players::domain::player::TeamId)
                        -> Result<Vec<crate::app::players::ports::PlayerProjection>, crate::app::players::ports::RepositoryError> { Ok(vec![]) }
                    async fn find_by_id(&self, _: &str)
                        -> Result<Option<crate::app::players::ports::PlayerProjection>, crate::app::players::ports::RepositoryError> { Ok(None) }
                }
                crate::app::players::context::PlayersContext {
                    repository:            Arc::new(FakePlayerRepo),
                    projection_repository: Arc::new(FakePlayerProjectionRepo),
                }
            },
            match_report: {
                struct FakeMrRepo;
                #[async_trait::async_trait]
                impl crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository for FakeMrRepo {
                    async fn append(&self, _: &str, _: &crate::app::match_report::domain::events::MatchReportDomainEvent, _: u64)
                        -> Result<u64, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(1) }
                    async fn append_many(&self, _: &str, _: Vec<crate::app::match_report::domain::events::MatchReportDomainEvent>, _: u64)
                        -> Result<u64, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(1) }
                    async fn find_by_id(&self, _: &str)
                        -> Result<Option<crate::app::match_report::domain::match_report_state::MatchReportState>, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(None) }
                    async fn find_id_by_pairing(&self, _: &str)
                        -> Result<Option<String>, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(None) }
                    async fn find_id_by_round_and_teams(&self, _: &str, _: &str, _: &str)
                        -> Result<Option<String>, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(None) }
                    async fn find_actions_by_match_and_side(&self, _: &str, _: crate::app::match_report::domain::value_objects::TeamSide)
                        -> Result<Vec<crate::app::match_report::domain::match_report_repository_port::MatchActionRow>, crate::app::match_report::domain::match_report_repository_port::RepositoryError> { Ok(vec![]) }
                }
                struct FakeCompDataPort;
                #[async_trait::async_trait]
                impl crate::app::match_report::ports::ICompetitionDataPort for FakeCompDataPort {
                    async fn is_competition_admin(&self, _: &str, _: &str) -> Result<bool, String> { Ok(false) }
                    async fn find_tier_rules_for_roster(&self, _: &str, _: &str) -> Option<crate::app::match_report::ports::TierRulesDto> { None }
                }
                struct FakeTeamDataPort;
                #[async_trait::async_trait]
                impl crate::app::match_report::ports::ITeamDataPort for FakeTeamDataPort {
                    async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> { Ok(true) }
                    async fn find_team_info(&self, _: &str) -> Option<crate::app::match_report::ports::TeamInfoDto> { None }
                    async fn find_team_value(&self, _: &str) -> Option<u32> { None }
                    async fn find_team_treasury(&self, _: &str) -> Option<u32> { None }
                    async fn find_journalier_position(&self, _: &str) -> Option<crate::app::match_report::ports::JournalierPositionDto> { None }
                }
                struct FakePlayerDataPort;
                #[async_trait::async_trait]
                impl crate::app::match_report::ports::IPlayerDataPort for FakePlayerDataPort {
                    async fn count_available_players(&self, _: &str) -> Result<usize, String> { Ok(0) }
                    async fn find_player_display(&self, _: &str) -> Option<String> { None }
                }
                crate::app::match_report::context::MatchReportContext {
                    match_report_repo: Arc::new(FakeMrRepo),
                    competition_data: Arc::new(FakeCompDataPort),
                    team_data: Arc::new(FakeTeamDataPort),
                    player_data: Arc::new(FakePlayerDataPort),
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

    struct FakeReferenceData;
    impl crate::app::team_creation::ports::IReferenceDataPort for FakeReferenceData {
        fn find_roster_definition(&self, _: &str) -> Option<crate::app::team_creation::ports::RosterDefinition> { None }
        fn list_staff_definitions(&self) -> Vec<crate::app::team_creation::ports::StaffDefinition> { vec![] }
        fn resolve_skill_cost(&self, _: &str, _: &str, _: &str) -> Option<crate::app::team_creation::ports::SkillCostResult> { None }
        fn resolve_skill_name(&self, _: &str) -> Option<String> { None }
        fn resolve_base_skills(&self, _: &str) -> Vec<String> { vec![] }
        fn skill_pricing_level_1(&self) -> Option<crate::app::team_creation::ports::SkillPricingDefinition> { None }
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
