//! La coupure des flux de l'appelant — `POST /app/realtime/test/close-my-streams`
//! (carte 646). **Réservée aux e2e** : elle répond 404 dès que `bypass_auth`
//! est faux, c'est-à-dire en production.
//!
//! # Pourquoi une route plutôt que `set_offline`
//!
//! Sous Chromium, `context.set_offline(True)` fait échouer les requêtes
//! nouvelles mais laisse couler une réponse déjà en cours de lecture : mesuré,
//! un flux SSE ouvert reçoit encore tous ses événements vingt secondes après.
//! Le seul moyen d'éprouver la reconnexion telle qu'elle se produit en
//! production — le serveur termine la réponse, le navigateur repasse en
//! `CONNECTING`, retente, ou reste `CLOSED` sur un 502 — est que le serveur
//! termine vraiment le flux.
//!
//! Elle ne ferme que les flux de **l'utilisateur qui appelle** : un test ne
//! coupe pas les onglets des autres, et la route ne donne prise sur personne.

use crate::app::auth::auth_backend::AuthSession;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn post_close_my_streams(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> Response {
    if !state.bypass_auth {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let closed = state.realtime.close_user_connections(&user.id);
    tracing::info!(closed, "flux fermés à la demande d'un test e2e");
    closed.to_string().into_response()
}
