//! Les messages du canal temps réel, et leur forme SSE.
//!
//! Chaque message devient un événement SSE **nommé** (`event: hello`) dont les
//! données sont du JSON. Le client du layout traduit chaque nom en événement
//! DOM sur `body` ; un nom qu'il ne connaît pas est ignoré.
//!
//! Le registre ne connaît que `RealtimeMessage` : la forme SSE vit ici, pour
//! qu'un autre transport ne change rien au registre.

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
}

impl RealtimeMessage {
    pub fn to_sse_event(&self) -> Event {
        match self {
            RealtimeMessage::Hello { connection_id } => {
                Event::default().event("hello").retry(RETRY_AFTER).data(
                    serde_json::json!({ "connection_id": connection_id.to_string() }).to_string(),
                )
            }
        }
    }
}
