use std::fmt::{Display, Formatter};
use crate::app::auth::domain::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{EventId, UserId};
use crate::lib::services::event_bus::event_bus::Event;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum AppEventKind {
    UserRegistered,
    UserBanned,
}

impl Display for AppEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEventKind::UserRegistered => write!(f, "UserRegistered"),
            AppEventKind::UserBanned    => write!(f, "UserBanned"),
        }
    }
}

#[derive(Debug)]
pub enum AppEvent {
    UserRegistered { event_id: EventId, user_id: UserId, user_name: CoachName, email: Email },
    UserBanned     { event_id: EventId, user_id: UserId },
}

impl Display for AppEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

impl Event for AppEvent {
    type Kind = AppEventKind;
    fn kind(&self) -> AppEventKind {
        match self {
            AppEvent::UserRegistered { .. } => AppEventKind::UserRegistered,
            AppEvent::UserBanned { .. }     => AppEventKind::UserBanned,
        }
    }
}