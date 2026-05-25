use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, EventId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_tags::{EventTag, EventTagName};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CompetitionsDomainEvent {
    CompetitionCreated {
        event_id:       EventId,
        competition_id: CompetitionId,
        space_id:       SpaceId,
        created_by:     CoachId,
        name:           CompetitionName,
        logo:           CloudinaryImage,
        admin_ids:      Vec<CoachId>,
    },
    CompetitionReady {
        event_id:       EventId,
        competition_id: CompetitionId,
        space_id:       SpaceId,
        finalized_by:   CoachId,
    },
}

pub const COMPETITION_CREATED: &str = "CompetitionCreated";
pub const COMPETITION_READY:   &str = "CompetitionReady";

impl CompetitionsDomainEvent {
    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::CompetitionCreated { .. } => COMPETITION_CREATED,
            Self::CompetitionReady   { .. } => COMPETITION_READY,
        }
    }

    fn competition_id(&self) -> CompetitionId {
        match self {
            Self::CompetitionCreated { competition_id, .. } => *competition_id,
            Self::CompetitionReady   { competition_id, .. } => *competition_id,
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::CompetitionCreated { space_id, competition_id, .. } |
            Self::CompetitionReady   { space_id, competition_id, .. } => vec![
                EventTag { name: EventTagName::Space,       value: space_id.to_string() },
                EventTag { name: EventTagName::Competition, value: competition_id.to_string() },
            ],
        }
    }

    pub fn to_app_event(&self) -> Option<CompetitionsAppEvent> {
        match self {
            Self::CompetitionReady { competition_id, space_id, .. } => {
                Some(CompetitionsAppEvent::CompetitionCreated {
                    event_id:       EventId::new(),
                    competition_id: *competition_id,
                    space_id:       *space_id,
                })
            }
            _ => None,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter:     self.competition_id().to_string(),
            event_type:  self.to_event_type().to_string(),
            tags:        serde_json::to_value(self.get_tags()).unwrap(),
            payload:     serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}