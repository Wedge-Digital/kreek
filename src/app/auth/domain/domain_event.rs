use serde::Deserialize;
use crate::app::auth::domain::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{EventId, UserId};

#[derive(Debug, Deserialize)]
pub enum AuthDomainEvent {
    UserRegistered              { event_id: EventId, user_id: UserId, user_name: CoachName, email: Email },
    UserPasswordResetRequested  { event_id: EventId, user_id: UserId },
    UserPasswordReset           { event_id: EventId, user_id: UserId, new_password: String },
    UserEmailVerified           { event_id: EventId, user_id: UserId },
    UserEmailVerificationFailed { event_id: EventId, user_id: UserId },
    UserLoggedIn                { event_id: EventId, user_id: UserId },
}