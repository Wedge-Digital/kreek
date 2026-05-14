use std::fmt::{Display, Formatter};
use crate::app::shared_kernel::team::BaseTeamInfo;
use crate::app::team_creation::domain::roster::Roster;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::lib::services::event_bus::event_bus::Event;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum TeamCreationEventKind {
    DraftTeamCreated,
    RulesetSelected,
    RosterSelected,
}

impl Display for TeamCreationEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamCreationEventKind::DraftTeamCreated => write!(f, "DraftTeamCreated"),
            TeamCreationEventKind::RulesetSelected  => write!(f, "RulesetSelected"),
            TeamCreationEventKind::RosterSelected   => write!(f, "RosterSelected"),
        }
    }
}

#[derive(Debug)]
pub enum TeamCreationEvent {
    DraftTeamCreated { base_team_info: BaseTeamInfo },
    RulesetSelected  { base_team_info: BaseTeamInfo, ruleset: Ruleset },
    RosterSelected   { base_team_info: BaseTeamInfo, ruleset: Ruleset, roster: Roster },
}

impl Display for TeamCreationEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

impl Event for TeamCreationEvent {
    type Kind = TeamCreationEventKind;
    fn kind(&self) -> TeamCreationEventKind {
        match self {
            TeamCreationEvent::DraftTeamCreated { .. } => TeamCreationEventKind::DraftTeamCreated,
            TeamCreationEvent::RulesetSelected  { .. } => TeamCreationEventKind::RulesetSelected,
            TeamCreationEvent::RosterSelected   { .. } => TeamCreationEventKind::RosterSelected,
        }
    }
}