use crate::app::auth::auth_backend::AuthSession;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

const BYPASS_AUTH_LEGACY_USER_ID: i32 = 1;

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
        match state
            .auth
            .user_repository
            .find_by_legacy_id(BYPASS_AUTH_LEGACY_USER_ID)
            .await
        {
            Ok(Some(user)) => {
                if auth_session.login(&user).await.is_ok() {
                    tracing::debug!(%path, "bypass_auth: login OK");
                    request.extensions_mut().insert(auth_session);
                } else {
                    tracing::warn!(%path, "bypass_auth: login échoué");
                }
            }
            Ok(None) => {
                tracing::warn!(%path, "bypass_auth: utilisateur legacy_id=1 introuvable");
            }
            Err(e) => {
                tracing::warn!(%path, error = %e, "bypass_auth: erreur lors de la recherche utilisateur");
            }
        }
    }

    tracing::debug!(%path, "bypass_auth: passage au handler suivant");
    let response = next.run(request).await;
    tracing::debug!(%path, status = %response.status(), "bypass_auth: réponse reçue");
    response
}
