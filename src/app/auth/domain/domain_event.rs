use serde::Deserialize;
use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;

#[derive(Debug, Deserialize)]
pub enum AuthDomainEvent {
    UserLoggedIn                { event_id: String, user_id: String },
    UserRegistered              { event_id: String, user_id: String, user_name: CoachName, email: Email },
    UserPasswordResetRequested  { event_id: String, user_id: String },
    UserPasswordReset           { event_id: String, user_id: String, new_password: String },
    UserEmailVerified           { event_id: String, user_id: String },
    UserEmailVerificationFailed { event_id: String, user_id: String },
}

pub const USER_LOGGED_IN: &str = "UserLoggedIn";
pub const USER_REGISTERED: &str = "UserRegistered";
