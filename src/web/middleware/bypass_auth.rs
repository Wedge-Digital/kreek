use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use crate::app::auth::auth_backend::{AuthSession, bypass_user};
use crate::state::AppState;

pub async fn bypass_auth_middleware(
    State(state): State<AppState>,
    mut auth_session: AuthSession,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    if state.bypass_auth && auth_session.user.is_none() {
        if auth_session.login(&bypass_user()).await.is_ok() {
            request.extensions_mut().insert(auth_session);
        }
    }
    next.run(request).await
}