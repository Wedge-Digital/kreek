extern crate core;

mod app;
mod cli;
mod config;
#[allow(special_module_name)]
pub mod common;
mod infrastructure;
mod state;
pub mod web;

use config::AppConfig;
use state::AppState;
use std::time::Duration;

use crate::app::auth::auth_backend::AuthBackend;
use crate::app::auth::context::AuthContext;
use crate::app::auth::routes::path;
use crate::app::competitions::context::CompetitionsContext;
use crate::app::news::context::NewsContext;
use crate::app::references::context::ReferencesContext;
use crate::app::spaces::context::SpacesContext;
use crate::app::team_creation::context::TeamCreationContext;
use crate::infrastructure::team_creation::competition_rules_adapter::CompetitionRulesAdapter;
use crate::infrastructure::team_creation::reference_data_adapter::ReferenceDataAdapter;
use crate::app::players::context::PlayersContext;
use crate::app::ranking::context::RankingContext;
use crate::app::teams::context::TeamsContext;
use crate::app::{auth, competitions, match_report, players, ranking, references, spaces, team_creation, teams};
use crate::common::event_listener::event_log_feeder;
use crate::common::services::email::ResendMailService;
use crate::common::services::event_bus::event_bus::new_bus;
use crate::common::session_store::DashMapStore;
use crate::web::middleware::bypass_auth::bypass_auth_middleware;
use crate::web::middleware::request_log::request_log;
use crate::web::middleware::require_auth::require_auth;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::{response::Redirect, routing::get, Router};
use axum_login::AuthManagerLayerBuilder;
use clap::Parser;
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tower_sessions::SessionManagerLayer;

#[derive(Parser)]
#[command(name = "kreek", about = "kreek — Blood Bowl league manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Lance le serveur HTTP
    Serve,
    /// Seed les comptes utilisateurs depuis un fichier JSON
    SeedAccounts {
        #[arg(long, default_value = "scripts/seed_accounts.json")]
        input: String,
    },
}

async fn init_pool(cfg: &AppConfig) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .min_connections(cfg.database.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.database.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(cfg.database.idle_timeout_seconds))
        .connect(&cfg.database.url)
        .await
        .expect("Impossible de se connecter à la base de données")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kreek=debug".into()),
        )
        .init();

    let cli = Cli::parse();

    let cfg =
        AppConfig::load().expect("Configuration invalide — vérifiez vos variables d'environnement");

    let pool = init_pool(&cfg).await;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => run_server(cfg, pool).await,
        Command::SeedAccounts { input } => {
            if let Err(e) = cli::seed_accounts::execute(&pool, &input).await {
                tracing::error!("seed-accounts failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

async fn run_server(cfg: AppConfig, pool: sqlx::PgPool) {
    let server_address = cfg.server_addr();

    let event_bus = new_bus();
    let app_event_bus = new_bus();

    event_log_feeder::init(&event_bus, pool.clone());
    auth::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    spaces::context::init_app_event_listeners(&app_event_bus, pool.clone());
    spaces::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    let competitions_team_info_port = Arc::new(
        crate::infrastructure::competitions::team_info_adapter::TeamInfoAdapter::new(
            Arc::new(crate::app::teams::io::repository::team_repository::TeamRepository::new(pool.clone())),
        ),
    );
    let competitions_match_day_repository = Arc::new(
        crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(pool.clone()),
    );
    competitions::context::init_listeners(
        &event_bus,
        app_event_bus.clone(),
        pool.clone(),
        competitions_match_day_repository.clone(),
        competitions_team_info_port.clone(),
    );
    team_creation::context::init_app_event_publisher(&event_bus, app_event_bus.clone());
    teams::context::init_listeners(&app_event_bus, pool.clone());
    let refs_for_players = references::context::ReferencesContext::new();
    let players_skill_catalog: Arc<dyn crate::app::players::ports::ISkillCatalogPort> = Arc::new(
        crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter::new(
            refs_for_players.repository.clone(),
        ),
    );
    let match_report_comp_data = Arc::new(
        crate::infrastructure::match_report::competition_data_adapter::CompetitionDataAdapter::new(
            Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
            Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
            refs_for_players.repository.clone(),
            Arc::new(crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(pool.clone())),
        ),
    );
    let match_report_team_data = Arc::new(
        crate::infrastructure::match_report::ref_team_data_adapter::RefTeamDataAdapter::new(
            Arc::new(crate::app::teams::io::repository::team_repository::TeamRepository::new(pool.clone())),
            refs_for_players.repository.clone(),
        ),
    );
    match_report::context::init_listeners(
        &event_bus,
        &app_event_bus,
        pool.clone(),
        match_report_comp_data.clone(),
        match_report_team_data.clone(),
    );
    players::context::init_listeners(
        &event_bus,
        &app_event_bus,
        pool.clone(),
        players_skill_catalog.clone(),
    );

    let ranking_competition_port = Arc::new(
        crate::infrastructure::ranking::competition_info_adapter::RankingCompetitionAdapter::new(
            Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
            competitions_team_info_port.clone(),
        ),
    );
    ranking::context::init_listeners(&app_event_bus, pool.clone(), ranking_competition_port.clone());

    let state = AppState {
        auth: AuthContext::new(&pool, event_bus.clone()),
        spaces: SpacesContext::new(&pool, event_bus.clone()),
        competitions: CompetitionsContext::new(
            &pool,
            event_bus.clone(),
            competitions_team_info_port,
            Arc::new(crate::infrastructure::competitions::reference_name_adapter::ReferenceNameAdapter::new(
                refs_for_players.repository.clone(),
            )),
            Arc::new(crate::infrastructure::competitions::space_member_adapter::SpaceMemberAdapter::new(
                Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
            )),
        ),
        news: NewsContext::new(&pool),
        references: ReferencesContext::new(),
        team_creation: TeamCreationContext::new(
            &pool,
            event_bus.clone(),
            Arc::new(ReferenceDataAdapter::new(refs_for_players.repository.clone())),
            Arc::new(CompetitionRulesAdapter::new(Arc::new(
                crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone()),
            ))),
            Arc::new(crate::infrastructure::team_creation::competition_display_adapter::CompetitionDisplayAdapter::new(
                Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
                Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
            )),
        ),
        match_report: {
            let comp_data = match_report_comp_data.clone();
            let team_data = match_report_team_data.clone();
            let player_data = Arc::new(
                crate::infrastructure::match_report::player_data_adapter::PlayerDataAdapter::new(
                    Arc::new(crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository::new(pool.clone())),
                ),
            );
            let coach_data = Arc::new(
                crate::infrastructure::match_report::coach_data_adapter::CoachDataAdapter::new(
                    Arc::new(crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository::new(pool.clone())),
                ),
            );
            let spp_calculator = Arc::new(
                crate::infrastructure::match_report::spp_calculator_adapter::SppCalculatorAdapter,
            );
            match_report::context::MatchReportContext::new(&pool, comp_data, team_data, player_data, coach_data, spp_calculator, event_bus.clone())
        },
        teams: {
            let player_count = Arc::new(
                crate::infrastructure::teams::player_count_adapter::PlayerCountAdapter::new(pool.clone()),
            );
            let journeyman_type = Arc::new(
                crate::infrastructure::teams::journeyman_type_adapter::JourneymanTypeAdapter::new(
                    refs_for_players.repository.clone(),
                ),
            );
            let roster_info = Arc::new(
                crate::infrastructure::teams::roster_info_adapter::RosterInfoAdapter::new(
                    refs_for_players.repository.clone(),
                ),
            );
            TeamsContext::new(&pool, player_count, journeyman_type, roster_info)
        },
        players: PlayersContext::new(
            &pool,
            players_skill_catalog.clone(),
            Arc::new(crate::infrastructure::players::team_roster_adapter::TeamRosterAdapter::new(
                Arc::new(crate::app::teams::io::repository::team_repository::TeamRepository::new(pool.clone())),
            )),
            Arc::new(crate::infrastructure::players::competition_admin_adapter::CompetitionAdminAdapter::new(
                Arc::new(crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone())),
            )),
            Arc::new(crate::infrastructure::players::space_member_adapter::SpaceMemberAdapter::new(
                Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
            )),
            event_bus.clone(),
        ),
        ranking: RankingContext::new(&pool, ranking_competition_port),
        email_service: Arc::new(ResendMailService::new(
            cfg.email.api_key,
            cfg.email.from,
            cfg.email.from_name,
        )),
        host_domain: cfg.host_domain,
        bypass_auth: cfg.bypass_auth,
        event_bus: event_bus.clone(),
        app_event_bus: app_event_bus.clone(),
    };

    let session_layer = SessionManagerLayer::new(DashMapStore::new());
    let auth_layer = AuthManagerLayerBuilder::new(
        AuthBackend::new(state.auth.user_repository.clone()),
        session_layer,
    )
    .build();

    let protected = Router::new()
        .merge(app::news::router::router())
        .merge(app::references::router::router())
        .merge(app::team_creation::router::router())
        .merge(app::players::router::router())
        .merge(app::ranking::router::router())
        .merge(app::teams::router::router())
        .merge(app::match_report::router::router())
        .merge(app::competitions::router::router())
        .merge(app::spaces::router::router())
        .merge(web::router::router())
        .route_layer(from_fn(require_auth))
        .route_layer(from_fn_with_state(state.clone(), bypass_auth_middleware));

    let auth_app = Router::new()
        .route("/", get(|| async { Redirect::to(path::AUTH_LAYOUT) }))
        .merge(app::auth::router::router())
        .merge(protected)
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let app = Router::new()
        .nest_service("/static", ServeDir::new("assets/static"))
        .merge(auth_app);

    #[cfg(debug_assertions)]
    let app = {
        #[derive(Clone, Copy)]
        struct NotHtmxRequest;
        impl tower_livereload::predicate::Predicate<axum::http::Request<axum::body::Body>>
            for NotHtmxRequest
        {
            fn check(&mut self, req: &axum::http::Request<axum::body::Body>) -> bool {
                !req.headers().contains_key("hx-request")
            }
        }
        Router::new()
            .nest_service("/ui", ServeDir::new("assets/templates"))
            .merge(app)
            .layer(from_fn(request_log))
            .layer(LiveReloadLayer::new().request_predicate(NotHtmxRequest))
    };

    let listener = tokio::net::TcpListener::bind(&server_address)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
