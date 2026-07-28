use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SpacesAppEvent {
    SpaceCreated {
        event_id: EventId,
        space_id: SpaceId,
        space_name: SpaceName,
        space_logo: CloudinaryImage,
        created_by: CoachId,
    },
    UserSubscribed {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
        space_profile: SpaceProfile,
    },
    UserUnsubscribed {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
}

impl SpacesAppEvent {
    pub const SPACE_CREATED: &'static str = "SpaceCreated";
    pub const USER_SUBSCRIBED: &'static str = "UserSubscribed";
    pub const USER_UNSUBSCRIBED: &'static str = "UserUnsubscribed";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SpaceCreated { .. } => Self::SPACE_CREATED,
            Self::UserSubscribed { .. } => Self::USER_SUBSCRIBED,
            Self::UserUnsubscribed { .. } => Self::USER_UNSUBSCRIBED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::SpaceCreated { space_id, .. } => space_id.to_string(),
            Self::UserSubscribed { user_id, .. } => user_id.to_string(),
            Self::UserUnsubscribed { user_id, .. } => user_id.to_string(),
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
