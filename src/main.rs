extern crate core;

mod app;
mod config;
mod lib;
mod state;

use std::time::Duration;
use config::AppConfig;
use state::AppState;

use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use std::sync::Arc;
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::io::repository::reset_token_repository::ResetTokenRepository;
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
        email_service:          Arc::new(ResendMailService::new(cfg.email.api_key, cfg.email.from, cfg.email.from_name)),
        host_domain:            cfg.host_domain,
    };

    let app = Router::new()
        .merge(app::auth::auth_router::router())
        .nest_service("/static", ServeDir::new("assets/static"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(debug_assertions)]
    let app = app
        .nest_service("/ui", ServeDir::new("assets/templates"))
        .layer(LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind(&server_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}