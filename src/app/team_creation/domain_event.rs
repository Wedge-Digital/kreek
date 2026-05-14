use crate::app::shared_kernel::team::BaseTeamInfo;
use crate::app::team_creation::domain::roster::Roster;
use crate::app::team_creation::domain::ruleset::Ruleset;

pub const DRAFT_TEAM_CREATED: &str = "DraftTeamCreated";
pub const RULESET_SELECTED:   &str = "RulesetSelected";
pub const ROSTER_SELECTED:    &str = "RosterSelected";

#[derive(Debug)]
pub enum TeamCreationEvent {
    DraftTeamCreated { base_team_info: BaseTeamInfo },
    RulesetSelected  { base_team_info: BaseTeamInfo, ruleset: Ruleset },
    RosterSelected   { base_team_info: BaseTeamInfo, ruleset: Ruleset, roster: Roster },
}