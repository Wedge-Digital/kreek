//! Le flux SSE d'un onglet — `GET /app/realtime/stream` (carte 642).
//!
//! À l'ouverture : enregistrement dans le registre, premier événement `hello`
//! portant l'identifiant de connexion, puis les messages du canal. Le
//! battement de cœur est le `KeepAlive` d'axum (un commentaire toutes les
//! 15 s), sous les 60 s du `proxy_read_timeout` par défaut de nginx.
//!
//! # Le seul nettoyage est le `Drop`
//!
//! Le stream porte une `ConnectionGuard`. Quand le navigateur ferme l'onglet,
//! hyper abandonne le corps de la réponse, le stream est détruit, et la garde
//! retire la connexion du registre. Aucun geste du client n'est attendu : un
//! client disparu sans fermer TCP est détecté à l'échec d'une écriture, que le
//! battement provoque.
//!
//! # Le journal
//!
//! `request_log` journalise « réponse envoyée » **à l'ouverture** : son span ne
//! couvre pas la vie du corps. La vraie fin est la ligne « flux fermé », émise
//! dans le span de la requête d'ouverture — capturé par la garde —, donc avec
//! le même `rid` et le même `coach`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::state::AppState;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::registry::{
    ConnectionId, ConnectionRegistry, ConnectionStats, OpenedConnection,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

/// `Sse` pose `content-type` et `cache-control`, pas cet en-tête : sans lui,
/// nginx tamponne la réponse et le flux n'arrive jamais (fiche E23, § 6).
const ACCEL_BUFFERING: (&str, &str) = ("x-accel-buffering", "no");

pub async fn get_realtime_stream(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> Response {
    // `require_auth` passe avant : ce cas ne se présente que si la route
    // sortait un jour de sa garde.
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let stream = open_stream(state.realtime.clone(), user.id, user.coach_name);
    let sse = Sse::new(stream).keep_alive(KeepAlive::default());
    ([ACCEL_BUFFERING], sse).into_response()
}

/// Enregistre la connexion, met `hello` en file et rend le flux.
pub fn open_stream(
    registry: Arc<ConnectionRegistry>,
    user_id: UserId,
    coach_name: CoachName,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let opened = registry.open(user_id, coach_name);
    let hello = RealtimeMessage::Hello {
        connection_id: opened.id,
    };
    // La file est neuve : `hello` y entre forcément.
    registry.deliver(&opened.id, hello);
    tracing::info!(
        conn = %opened.id,
        open = registry.open_count(),
        tabs = registry.user_connection_count(&user_id),
        "flux ouvert"
    );
    build_stream(registry, opened)
}

/// Le flux vit tant que son corps est lu ; sa garde meurt avec lui.
fn build_stream(
    registry: Arc<ConnectionRegistry>,
    opened: OpenedConnection,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let guard = ConnectionGuard::new(registry, opened.id, opened.stats);
    stream::unfold(
        (opened.receiver, guard),
        |(mut receiver, guard)| async move {
            let message = receiver.recv().await?;
            guard.stats.record_delivered();
            Some((Ok(message.to_sse_event()), (receiver, guard)))
        },
    )
}

/// Retire la connexion du registre quand le flux est détruit, et le dit.
struct ConnectionGuard {
    registry: Arc<ConnectionRegistry>,
    id: ConnectionId,
    stats: Arc<ConnectionStats>,
    opened_at: Instant,
    span: tracing::Span,
}

impl ConnectionGuard {
    fn new(
        registry: Arc<ConnectionRegistry>,
        id: ConnectionId,
        stats: Arc<ConnectionStats>,
    ) -> Self {
        Self {
            registry,
            id,
            stats,
            opened_at: Instant::now(),
            span: tracing::Span::current(),
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let left = self.registry.close(&self.id);
        let _entered = self.span.enter();
        if let Some(match_report_id) = left {
            // L'onglet fermé quitte son match : ses autres présents l'apprennent.
            tracing::debug!(conn = %self.id, %match_report_id, role = "none", "présence");
            self.registry
                .notify_presence_changed(&match_report_id, None);
        }
        tracing::info!(
            conn = %self.id,
            duration_s = self.opened_at.elapsed().as_secs(),
            delivered = self.stats.delivered(),
            dropped = self.stats.dropped(),
            open = self.registry.open_count(),
            "flux fermé"
        );
    }
}
