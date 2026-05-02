use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::domain_event::DomainEvent;
use crate::app::shared_kernel::team::BaseTeamInfo;
use crate::app::team_creation::domain::ruleset::Ruleset;


#[derive(Debug, Deserialize, Serialize)]
struct RulesetSelectedEvent {
    pub base_team_info: BaseTeamInfo,
    pub ruleset: Ruleset,
}

impl DomainEvent for RulesetSelectedEvent {
    fn event_type() -> &'static str { "RulesetSelectedEvent" }
    fn version() -> &'static str { "1.0" }
    fn schema()     -> &'static str { "/schemas/team_creation" }
}
