use crate::app::auth::auth_backend::{bypass_user, AuthSession};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn bypass_auth_middleware(
    State(state): State<AppState>,
    mut auth_session: AuthSession,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();

    tracing::debug!(%path, has_user = auth_session.user.is_some(), "bypass_auth: entrée");

    if state.bypass_auth && auth_session.user.is_none() {
        tracing::debug!(%path, "bypass_auth: login automatique");
        if auth_session.login(&bypass_user()).await.is_ok() {
            tracing::debug!(%path, "bypass_auth: login OK");
            request.extensions_mut().insert(auth_session);
        } else {
            tracing::warn!(%path, "bypass_auth: login échoué");
        }
    }

    tracing::debug!(%path, "bypass_auth: passage au handler suivant");
    let response = next.run(request).await;
    tracing::debug!(%path, status = %response.status(), "bypass_auth: réponse reçue");
    response
}
