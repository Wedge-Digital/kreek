//! Les messages du canal temps réel, et leur forme SSE.
//!
//! Chaque message devient un événement SSE **nommé** (`event: hello`) dont les
//! données sont du JSON. Le client du layout traduit chaque nom en événement
//! DOM sur `body` ; un nom qu'il ne connaît pas est ignoré.
//!
//! Le registre ne connaît que `RealtimeMessage` : la forme SSE vit ici, pour
//! qu'un autre transport ne change rien au registre.

use crate::app::shared_kernel::identity::ids::EntityId;
use crate::web::realtime::registry::ConnectionId;
use axum::response::sse::Event;
use std::time::Duration;

/// Délai de reconnexion demandé au navigateur après une coupure.
pub const RETRY_AFTER: Duration = Duration::from_millis(5000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimeMessage {
    /// Le premier message de chaque flux : l'identifiant de la connexion, que
    /// l'onglet garde pour se désigner (présence, carte 644).
    Hello { connection_id: ConnectionId },
    // ── Carte 645 : les notifications ──
    /// Un toast composé par l'hôte depuis un fait de match (`toast.rs`).
    Toast(ToastPayload),
    /// Des messages ont pu être perdus (`Lagged`) : les widgets abonnés se
    /// rechargent, comme à une reconnexion.
    Resync,
    // ── Carte 644 : la présence ──
    /// Quelqu'un est arrivé sur ce match, en est parti, ou y a changé de rôle
    /// (carte 644) : le widget de présence se recharge.
    PresenceChanged { match_report_id: EntityId },
}

/// Ce que le layout passe à `kreekToast(title, 'success', { subtitle, link })`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ToastPayload {
    pub title: String,
    pub subtitle: Option<String>,
    pub link: Option<String>,
}

impl RealtimeMessage {
    pub fn to_sse_event(&self) -> Event {
        match self {
            RealtimeMessage::Hello { connection_id } => {
                Event::default().event("hello").retry(RETRY_AFTER).data(
                    serde_json::json!({ "connection_id": connection_id.to_string() }).to_string(),
                )
            }
            // ── Carte 645 ──
            RealtimeMessage::Toast(payload) => Event::default()
                .event("toast")
                .data(serde_json::to_string(payload).unwrap_or_default()),
            RealtimeMessage::Resync => Event::default().event("resync").data("{}"),
            // ── Carte 644 ──
            RealtimeMessage::PresenceChanged { match_report_id } => {
                Event::default().event("presence").data(
                    serde_json::json!({ "match_report_id": match_report_id.to_string() })
                        .to_string(),
                )
            }
        }
    }
}
