extern crate core;

mod app;
mod config;
#[allow(special_module_name)]
pub mod common;
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
use crate::app::players::context::PlayersContext;
use crate::app::teams::context::TeamsContext;
use crate::app::{auth, competitions, players, references, spaces, team_creation, teams};
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
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tower_sessions::SessionManagerLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kreek=debug".into()),
        )
        .init();

    let cfg =
        AppConfig::load().expect("Configuration invalide — vérifiez vos variables d'environnement");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .min_connections(cfg.database.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.database.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(cfg.database.idle_timeout_seconds))
        .connect(&cfg.database.url)
        .await
        .expect("Impossible de se connecter à la base de données");

    let server_address = cfg.server_addr();

    let event_bus = new_bus();
    let app_event_bus = new_bus();

    event_log_feeder::init(&event_bus, pool.clone());
    auth::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    spaces::context::init_app_event_listeners(&app_event_bus, pool.clone());
    spaces::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    competitions::context::init_app_event_publisher(&event_bus, app_event_bus.clone());
    team_creation::context::init_app_event_publisher(&event_bus, app_event_bus.clone());
    teams::context::init_listeners(&app_event_bus, pool.clone());
    let refs_for_players = references::context::ReferencesContext::new();
    players::context::init_listeners(
        &app_event_bus,
        pool.clone(),
        refs_for_players.repository.clone(),
    );

    let state = AppState {
        auth: AuthContext::new(&pool, event_bus.clone()),
        spaces: SpacesContext::new(&pool, event_bus.clone()),
        competitions: CompetitionsContext::new(&pool, event_bus.clone()),
        news: NewsContext::new(&pool),
        references: ReferencesContext::new(),
        team_creation: TeamCreationContext::new(&pool, event_bus.clone()),
        teams:   TeamsContext::new(&pool),
        players: PlayersContext::new(&pool),
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
        AuthBackend::new(state.auth.user_repository.clone(), cfg.bypass_auth),
        session_layer,
    )
    .build();

    let protected = Router::new()
        .merge(app::news::router::router())
        .merge(app::references::router::router())
        .merge(app::team_creation::router::router())
        .merge(app::players::router::router())
        .merge(app::teams::router::router())
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
        // Exclude HTMX fragment requests from livereload script injection.
        // Without this, every HTMX swap injects a new <script> that opens a
        // persistent SSE connection, exhausting the browser's 6-connection-per-origin
        // limit (HTTP/1.1) after just two open tabs.
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
