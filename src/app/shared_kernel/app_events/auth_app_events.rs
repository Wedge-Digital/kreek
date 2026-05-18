use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::app::shared_kernel::email::Email;
use crate::lib::event_envelope::EventEnvelope;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AuthAppEvent {
    AccountCreated      { event_id: EventId, user_id: UserId, user_name: CoachName, email: Email },
    UserBanned          { event_id: EventId, user_id: UserId },
}

impl AuthAppEvent {
    pub const ACCOUNT_CREATED: &'static str = "AccountCreated";
    pub const USER_BANNED:      &'static str = "UserBanned";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::AccountCreated { .. } => Self::ACCOUNT_CREATED,
            Self::UserBanned { .. }     => Self::USER_BANNED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::AccountCreated { user_id, .. } => user_id.to_string(),
            Self::UserBanned { user_id, .. }     => user_id.to_string(),
        };
        EventEnvelope {
            event_id:    EventId::new().to_string(),
            emitter,
            event_type:  self.event_type().to_string(),
            tags:        serde_json::json!([]),
            payload:     serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}