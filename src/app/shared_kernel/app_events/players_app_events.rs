use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Franchit la frontière `players` → `teams`.
///
/// `InitialRosterCompleted` résout une course : les deux BCs s'abonnent au même
/// app event `TeamCreated`, dans deux tâches indépendantes. Sans ce signal,
/// `teams` peut atteindre `ReadyToPlay` et recalculer sa valeur avant que
/// `players` ait inséré le moindre joueur — la TV vaudrait alors zéro, ou pire
/// une valeur partielle, plausible et fausse.
///
/// `teams` pourrait valoriser lui-même le payload de `TeamCreated`, qui porte
/// déjà lignes de roster et compétences. Écarté : il réimplémenterait la règle
/// de valorisation de `players`, la duplication même qui avait produit les deux
/// tables divergentes corrigées par la carte 249.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayersAppEvent {
    InitialRosterCompleted { team_id: String, player_count: u32 },
}

impl PlayersAppEvent {
    pub const INITIAL_ROSTER_COMPLETED: &'static str = "PlayersInitialRosterCompleted";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::InitialRosterCompleted { .. } => Self::INITIAL_ROSTER_COMPLETED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::InitialRosterCompleted { team_id, .. } => team_id.clone(),
        };
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter,
            event_type: self.event_type().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
