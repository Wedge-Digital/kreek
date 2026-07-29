use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::shared_kernel::identity::spaces_app_events::SpacesAppEvent;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_tags::EventTag;
use crate::common::services::event_bus::event_tags::EventTagName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SpacesDomainEvent {
    SpaceCreated {
        event_id: EventId,
        created_by: CoachId,
        space_name: SpaceName,
        space_logo: CloudinaryImage,
        space_id: SpaceId,
    },
    UserInvitedInSpace {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    UserSubscribedToSpace {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
        space_profile: SpaceProfile,
    },
    UserPromotedToSpaceAdmin {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    SpaceArchived {
        event_id: EventId,
        space_id: SpaceId,
    },
}

pub const SPACE_CREATED: &str = "SpaceCreated";
pub const USER_SUBSCRIBED_TO_SPACE: &str = "UserRegisteredInSpace";
pub const USER_INVITED_IN_SPACE: &str = "UserRegisteredInSpace";
pub const USER_PROMOTED_TO_SPACE_ADMIN: &str = "UserPromotedToSpaceAdmin";
pub const SPACE_ARCHIVED: &str = "SpaceArchived";

impl SpacesDomainEvent {
    pub fn to_app_event(&self) -> Option<SpacesAppEvent> {
        match self {
            SpacesDomainEvent::SpaceCreated {
                space_name,
                space_id,
                space_logo,
                created_by,
                ..
            } => Some(SpacesAppEvent::SpaceCreated {
                event_id: EventId::new(),
                space_id: *space_id,
                space_name: space_name.clone(),
                space_logo: space_logo.clone(),
                created_by: *created_by,
            }),
            SpacesDomainEvent::UserSubscribedToSpace {
                user_id,
                space_id,
                space_profile,
                ..
            } => Some(SpacesAppEvent::UserSubscribed {
                event_id: EventId::new(),
                user_id: user_id.clone(),
                space_id: space_id.clone(),
                space_profile: space_profile.clone(),
            }),
            _ => None,
        }
    }

    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::SpaceCreated { .. } => SPACE_CREATED,
            Self::UserInvitedInSpace { .. } => USER_INVITED_IN_SPACE,
            Self::UserSubscribedToSpace { .. } => USER_SUBSCRIBED_TO_SPACE,
            Self::UserPromotedToSpaceAdmin { .. } => USER_PROMOTED_TO_SPACE_ADMIN,
            Self::SpaceArchived { .. } => SPACE_ARCHIVED,
        }
    }

    fn space_id(&self) -> CoachId {
        match self {
            Self::SpaceCreated { space_id, .. } => *space_id,
            Self::UserInvitedInSpace { space_id, .. } => *space_id,
            Self::UserSubscribedToSpace { space_id, .. } => *space_id,
            Self::UserPromotedToSpaceAdmin { space_id, .. } => *space_id,
            Self::SpaceArchived { space_id, .. } => *space_id,
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::SpaceCreated { space_id, .. } => vec![EventTag {
                name: EventTagName::Space,
                value: space_id.to_string(),
            }],
            Self::UserInvitedInSpace {
                space_id, user_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::User,
                    value: user_id.to_string(),
                },
            ],
            Self::UserSubscribedToSpace {
                space_id, user_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::User,
                    value: user_id.to_string(),
                },
            ],
            Self::UserPromotedToSpaceAdmin { space_id, .. } => vec![EventTag {
                name: EventTagName::Space,
                value: space_id.to_string(),
            }],
            Self::SpaceArchived { space_id, .. } => vec![EventTag {
                name: EventTagName::Space,
                value: space_id.to_string(),
            }],
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.space_id().to_string(),
            event_type: self.to_event_type().parse().unwrap(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
