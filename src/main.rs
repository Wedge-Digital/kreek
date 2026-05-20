extern crate core;

mod app;
mod config;
mod state;
pub mod web;
#[allow(special_module_name)]
pub mod lib;

use std::time::Duration;
use config::AppConfig;
use state::AppState;

use axum::{response::Redirect, routing::get, Router};
use crate::app::auth::routes::path;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use std::sync::Arc;
use axum::middleware::{from_fn, from_fn_with_state};
use axum_login::AuthManagerLayerBuilder;
use tower_sessions::SessionManagerLayer;
use crate::lib::session_store::DashMapStore;
use crate::app::auth::auth_backend::AuthBackend;
use crate::web::middleware::bypass_auth::bypass_auth_middleware;
use crate::web::middleware::require_auth::require_auth;
use crate::web::middleware::request_log::request_log;
use crate::app::auth::context::AuthContext;
use crate::app::{auth, competitions, spaces};
use crate::app::competitions::context::CompetitionsContext;
use crate::app::spaces::context::SpacesContext;
use crate::lib::services::email::ResendMailService;
use crate::lib::services::event_bus::event_bus::new_bus;
use crate::lib::event_listener::event_log_feeder;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kreek=debug".into()),
        )
        .init();

    let cfg = AppConfig::load()
        .expect("Configuration invalide — vérifiez vos variables d'environnement");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .min_connections(cfg.database.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.database.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(cfg.database.idle_timeout_seconds))
        .connect(&cfg.database.url)
        .await
        .expect("Impossible de se connecter à la base de données");

    let server_address = cfg.server_addr();

    let event_bus     = new_bus();
    let app_event_bus = new_bus();

    event_log_feeder::init(&event_bus, pool.clone());
    auth::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    spaces::context::init_app_event_listeners(&app_event_bus, pool.clone());
    spaces::context::init_app_event_publisher(&event_bus, app_event_bus.clone());

    competitions::context::init_app_event_listeners(&app_event_bus, pool.clone());

    let state = AppState {
        auth:          AuthContext::new(&pool, event_bus.clone()),
        spaces:        SpacesContext::new(&pool, event_bus.clone()),
        competitions:  CompetitionsContext::new(&pool, event_bus.clone()),
        email_service: Arc::new(ResendMailService::new(cfg.email.api_key, cfg.email.from, cfg.email.from_name)),
        host_domain:   cfg.host_domain,
        bypass_auth:   cfg.bypass_auth,
        event_bus:     event_bus.clone(),
        app_event_bus: app_event_bus.clone(),
    };

    let session_layer = SessionManagerLayer::new(DashMapStore::new());
    let auth_layer = AuthManagerLayerBuilder::new(
        AuthBackend::new(state.auth.user_repository.clone(), cfg.bypass_auth),
        session_layer,
    ).build();

    let protected = Router::new()
        .merge(app::news::router::router())
        .merge(app::team_creation::router::router())
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
    let app = Router::new()
        .nest_service("/ui", ServeDir::new("assets/templates"))
        .merge(app)
        .layer(from_fn(request_log))
        .layer(LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind(&server_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}