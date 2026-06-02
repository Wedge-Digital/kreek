use crate::app::shared_kernel::app_events::team_creation_app_events::TeamCreationAppEvent;
use crate::app::shared_kernel::common_types::EventId;
use crate::app::shared_kernel::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_tags::{EventTag, EventTagName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TeamCreationDomainEvent {
    TeamSubmitted {
        event_id: EventId,
        team_id: String,
        space_id: String,
        team_name: String,
        roster_id: String,
        roster_name: String,
        coach_id: String,
        coach_name: String,
        treasury: u32,
        rerolls: RerollCount,
        apothecaries: ApothecaryCount,
        assistants: AssistantCount,
        cheerleaders: CheerleaderCount,
        fans_factor: u8,
    },
}

pub const TEAM_SUBMITTED: &str = "TeamSubmitted";

impl TeamCreationDomainEvent {
    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::TeamSubmitted { .. } => TEAM_SUBMITTED,
        }
    }

    fn team_id(&self) -> &str {
        match self {
            Self::TeamSubmitted { team_id, .. } => team_id,
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::TeamSubmitted {
                space_id, team_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.clone(),
                },
                EventTag {
                    name: EventTagName::Team,
                    value: team_id.clone(),
                },
            ],
        }
    }

    pub fn to_app_event(&self) -> Option<TeamCreationAppEvent> {
        match self {
            Self::TeamSubmitted {
                team_id,
                space_id,
                team_name,
                roster_id,
                roster_name,
                coach_id,
                coach_name,
                treasury,
                rerolls,
                apothecaries,
                assistants,
                cheerleaders,
                fans_factor,
                ..
            } => Some(TeamCreationAppEvent::TeamCreated {
                event_id: EventId::new().to_string(),
                team_id: team_id.clone(),
                space_id: space_id.clone(),
                team_name: team_name.clone(),
                roster_id: roster_id.clone(),
                roster_name: roster_name.clone(),
                coach_id: coach_id.clone(),
                coach_name: coach_name.clone(),
                treasury: *treasury,
                rerolls: rerolls.0,
                apothecaries: apothecaries.0,
                assistants: assistants.0,
                cheerleaders: cheerleaders.0,
                fans_factor: *fans_factor,
            }),
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.team_id().to_string(),
            event_type: self.to_event_type().to_string(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
