use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::routes::path;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::body::Body;

pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    if let Err(e) = auth_session.logout().await {
        tracing::error!("logout: session flush failed: {e}");
    }

    Response::builder()
        .header(header::LOCATION, path::AUTH_LAYOUT)
        .status(axum::http::StatusCode::SEE_OTHER)
        .body(Body::empty())
        .unwrap()
}