use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SpacesDomainEvent {
    SpaceCreated                { event_id: String, created_by: String, space_name: String, space_logo: String, space_id: String },
    UserInvitedInSpace          { event_id: String, user_id: String, user_name: CoachName, email: Email },
    UserSubscribedToSpace       { event_id: String, space_id: String, user_id: String, user_name: CoachName, email: Email },
    UserPromotedToSpaceAdmin    { event_id: String, user_id: String },
    SpaceArchived               { event_id: String, space_id: String },
}

pub const SPACE_CREATED: &str = "SpaceCreated";
pub const USER_SUBSCRIBED_TO_SPACE: &str = "UserRegisteredInSpace";
pub const USER_INVITED_IN_SPACE: &str = "UserRegisteredInSpace";
pub const USER_PROMOTED_TO_SPACE_ADMIN: &str = "UserPromotedToSpaceAdmin";
pub const SPACE_ARCHIVED: &str = "SpaceArchived";
