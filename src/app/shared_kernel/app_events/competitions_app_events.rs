use crate::app::shared_kernel::common_types::{CompetitionId, EventId, SpaceId};
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CompetitionsAppEvent {
    CompetitionCreated {
        event_id: EventId,
        competition_id: CompetitionId,
        space_id: SpaceId,
    },
    PairingCreated {
        event_id: EventId,
        pairing_id: String,
        competition_id: String,
        season_id: String,
        round_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
    },
    PairingDeleted {
        event_id: EventId,
        pairing_id: String,
    },
}

impl CompetitionsAppEvent {
    pub const COMPETITION_CREATED: &'static str = "CompetitionCreated";
    pub const PAIRING_CREATED: &'static str = "PairingCreated";
    pub const PAIRING_DELETED: &'static str = "PairingDeleted";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::CompetitionCreated { .. } => Self::COMPETITION_CREATED,
            Self::PairingCreated { .. } => Self::PAIRING_CREATED,
            Self::PairingDeleted { .. } => Self::PAIRING_DELETED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::CompetitionCreated { competition_id, .. } => competition_id.to_string(),
            Self::PairingCreated { pairing_id, .. } => pairing_id.clone(),
            Self::PairingDeleted { pairing_id, .. } => pairing_id.clone(),
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
