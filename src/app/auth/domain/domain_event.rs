use crate::app::shared_kernel::app_events::auth_app_events::AuthAppEvent;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CoachId, EventId};
use crate::app::shared_kernel::email::Email;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_tags::EventTag;
use crate::common::services::event_bus::event_tags::EventTagName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum AuthDomainEvent {
    AccountCreated {
        event_id: EventId,
        user_id: CoachId,
        user_name: CoachName,
        email: Email,
    },
    UserLoggedIn {
        event_id: EventId,
        user_id: CoachId,
    },
    UserPasswordResetRequested {
        event_id: EventId,
        user_id: CoachId,
    },
    UserPasswordReset {
        event_id: EventId,
        user_id: CoachId,
        new_password: String,
    },
    UserEmailVerified {
        event_id: EventId,
        user_id: CoachId,
    },
    UserEmailVerificationFailed {
        event_id: EventId,
        user_id: CoachId,
    },
}

pub const ACCOUNT_CREATED: &str = "AccountCreated";
pub const USER_LOGGED_IN: &str = "UserLoggedIn";
pub const USER_PASSWORD_RESET_REQUESTED: &str = "UserPasswordResetRequested";
pub const USER_PASSWORD_RESET: &str = "UserPasswordReset";
pub const USER_EMAIL_VERIFIED: &str = "UserEmailVerified";
pub const USER_EMAIL_VERIFICATION_FAILED: &str = "UserEmailVerificationFailed";

impl AuthDomainEvent {
    pub fn to_app_event(&self) -> Option<AuthAppEvent> {
        match self {
            AuthDomainEvent::AccountCreated {
                user_id,
                user_name,
                email,
                ..
            } => Some(AuthAppEvent::AccountCreated {
                event_id: EventId::new(),
                user_id: *user_id,
                user_name: user_name.clone(),
                email: email.clone(),
            }),
            _ => None,
        }
    }

    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::UserLoggedIn { .. } => USER_LOGGED_IN,
            Self::AccountCreated { .. } => ACCOUNT_CREATED,
            Self::UserPasswordResetRequested { .. } => USER_PASSWORD_RESET_REQUESTED,
            Self::UserPasswordReset { .. } => USER_PASSWORD_RESET,
            Self::UserEmailVerified { .. } => USER_EMAIL_VERIFIED,
            Self::UserEmailVerificationFailed { .. } => USER_EMAIL_VERIFICATION_FAILED,
        }
    }

    fn user_id(&self) -> CoachId {
        match self {
            Self::UserLoggedIn { user_id, .. } => *user_id,
            AuthDomainEvent::AccountCreated { user_id, .. } => *user_id,
            AuthDomainEvent::UserPasswordResetRequested { user_id, .. } => *user_id,
            AuthDomainEvent::UserPasswordReset { user_id, .. } => *user_id,
            AuthDomainEvent::UserEmailVerified { user_id, .. } => *user_id,
            AuthDomainEvent::UserEmailVerificationFailed { user_id, .. } => *user_id,
        }
    }

    pub fn get_tags(&self) -> EventTag {
        match self {
            _ => EventTag {
                name: EventTagName::User,
                value: self.user_id().to_string(),
            },
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.user_id().to_string(),
            event_type: self.to_event_type().parse().unwrap(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
