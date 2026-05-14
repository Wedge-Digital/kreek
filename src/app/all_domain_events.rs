use std::fmt::{Display, Formatter};
use crate::app::auth::domain_event::{AuthDomainEvent, AuthDomainEventKind};
use crate::app::team_creation::domain_event::{TeamCreationEvent, TeamCreationEventKind};
use crate::lib::services::event_bus::event_bus::Event;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum AllDomainEventKind {
    Auth(AuthDomainEventKind),
    TeamCreation(TeamCreationEventKind),
}

impl Display for AllDomainEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AllDomainEventKind::Auth(k)         => write!(f, "Auth::{}", k),
            AllDomainEventKind::TeamCreation(k) => write!(f, "TeamCreation::{}", k),
        }
    }
}

#[derive(Debug)]
pub enum AllDomainEvents {
    Auth(AuthDomainEvent),
    TeamCreation(TeamCreationEvent),
}

impl Display for AllDomainEvents {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

impl Event for AllDomainEvents {
    type Kind = AllDomainEventKind;
    fn kind(&self) -> AllDomainEventKind {
        match self {
            AllDomainEvents::Auth(e)         => AllDomainEventKind::Auth(e.kind()),
            AllDomainEvents::TeamCreation(e) => AllDomainEventKind::TeamCreation(e.kind()),
        }
    }
}