use crate::app::shared_kernel::domain_event::DomainEvent;
use crate::app::shared_kernel::bloodbowl::team::BaseTeamInfo;
use crate::app::team_creation::domain::ruleset::Ruleset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DraftTeamCreatedEvent {
    pub base_team_info: BaseTeamInfo,
}

impl DomainEvent for DraftTeamCreatedEvent {
    fn event_type() -> &'static str { "DraftTeamCreatedEvent" }
    fn version() -> &'static str { "1.0" }
    fn schema()     -> &'static str { "/schemas/team_creation" }
}