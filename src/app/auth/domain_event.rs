use std::fmt::{Display, Formatter};
use serde::Deserialize;
use crate::app::auth::domain::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::lib::services::event_bus::event_bus::Event;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum AuthDomainEventKind {
    UserRegistered,
    UserPasswordResetRequested,
    UserPasswordReset,
    UserEmailVerified,
    UserEmailVerificationFailed,
    UserLoggedIn,
    UserBanned,
}

impl Display for AuthDomainEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthDomainEventKind::UserRegistered => write!(f, "UserRegistered"),
            AuthDomainEventKind::UserPasswordResetRequested => write!(f, "UserPasswordResetRequested"),
            AuthDomainEventKind::UserPasswordReset => write!(f, "UserPasswordReset"),
            AuthDomainEventKind::UserEmailVerified => write!(f, "UserEmailVerified"),
            AuthDomainEventKind::UserEmailVerificationFailed => write!(f, "UserEmailVerificationFailed"),
            AuthDomainEventKind::UserLoggedIn => write!(f, "UserLoggedIn"),
            AuthDomainEventKind::UserBanned => write!(f, "UserBanned"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum AuthDomainEvent {
    UserRegistered { event_id: EventId, user_id: UserId, user_name: CoachName, email: Email},
    UserPasswordResetRequested{ event_id: EventId, user_id: UserId} ,
    UserPasswordReset { event_id: EventId, user_id: UserId, new_password: String},
    UserEmailVerified { event_id: EventId, user_id: UserId},
    UserEmailVerificationFailed { event_id: EventId, user_id: UserId},
    UserLoggedIn {event_id: EventId, user_id: UserId},
}

impl Display for AuthDomainEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

impl Event for AuthDomainEvent {
    type Kind = AuthDomainEventKind;
    fn kind(&self) -> AuthDomainEventKind {
        match self {
            AuthDomainEvent::UserRegistered { .. } => AuthDomainEventKind::UserRegistered,
            AuthDomainEvent::UserPasswordResetRequested { .. } => AuthDomainEventKind::UserPasswordResetRequested,
            AuthDomainEvent::UserPasswordReset { .. } => AuthDomainEventKind::UserPasswordReset,
            AuthDomainEvent::UserEmailVerified { .. } => AuthDomainEventKind::UserEmailVerified,
            AuthDomainEvent::UserEmailVerificationFailed { .. } => AuthDomainEventKind::UserEmailVerificationFailed,
            AuthDomainEvent::UserLoggedIn { .. } => AuthDomainEventKind::UserLoggedIn,
        }
    }
}


