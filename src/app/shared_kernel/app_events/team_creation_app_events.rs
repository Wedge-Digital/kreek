use crate::app::shared_kernel::common_types::EventId;
use crate::lib::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TeamCreationAppEvent {
    TeamCreated {
        event_id: String,
        team_id: String,
        space_id: String,
        team_name: String,
        roster_id: String,
        roster_name: String,
        coach_id: String,
        coach_name: String,
        treasury: u32,
        rerolls: u8,
        apothecaries: u8,
        assistants: u8,
        cheerleaders: u8,
        fans_factor: u8, // améliorations achetées ; BC teams ajoute +1 (base)
    },
}

impl TeamCreationAppEvent {
    pub const TEAM_CREATED: &'static str = "TeamCreated";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TeamCreated { .. } => Self::TEAM_CREATED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::TeamCreated { team_id, .. } => team_id.clone(),
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
