extern crate core;

mod app;
mod config;
mod lib;
mod state;
pub mod web;

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
use crate::app::auth::io::repository::reset_token_repository::ResetTokenRepository;
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::lib::services::email::ResendMailService;

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

    let state = AppState {
        user_repository:        Arc::new(UserRepository::new(pool.clone())),
        reset_token_repository: Arc::new(ResetTokenRepository::new(pool.clone())),
        space_repository:       Arc::new(SpaceRepository::new(pool.clone())),
        email_service:          Arc::new(ResendMailService::new(cfg.email.api_key, cfg.email.from, cfg.email.from_name)),
        host_domain:            cfg.host_domain,
        bypass_auth:            cfg.bypass_auth,
    };

    let session_layer = SessionManagerLayer::new(DashMapStore::new());
    let auth_layer = AuthManagerLayerBuilder::new(
        AuthBackend::new(state.user_repository.clone(), cfg.bypass_auth),
        session_layer,
    ).build();

    let protected = Router::new()
        .merge(app::news::router::router())
        .merge(app::team_creation::router::router())
        .merge(app::competition::router::router())
        .merge(app::spaces::router::router())
        .merge(web::router::router())
        .route_layer(from_fn(require_auth))
        .route_layer(from_fn_with_state(state.clone(), bypass_auth_middleware));

    let app = Router::new()
        .route("/", get(|| async { Redirect::to(path::AUTH_LAYOUT) }))
        .merge(app::auth::router::router())
        .merge(protected)
        .nest_service("/static", ServeDir::new("assets/static"))
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(debug_assertions)]
    let app = app
        .nest_service("/ui", ServeDir::new("assets/templates"))
        .layer(LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind(&server_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}