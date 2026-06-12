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
}

impl CompetitionsAppEvent {
    pub const COMPETITION_CREATED: &'static str = "CompetitionCreated";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::CompetitionCreated { .. } => Self::COMPETITION_CREATED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::CompetitionCreated { competition_id, .. } => competition_id.to_string(),
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
