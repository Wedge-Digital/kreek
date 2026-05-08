use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, UserId};
use crate::app::shared_kernel::team::{TeamId, TeamName};
use crate::app::team_creation::domain::roster::Roster;
use crate::app::team_creation::domain::ruleset::Ruleset;

pub struct RegisterNewTeamCommand {
    pub team_name: TeamName,
    pub coach_id: CoachId,
    pub logo_url: Option<CloudinaryImage>,
    pub created_by: UserId,
}

pub struct SelectRulesetCommand {
    pub team_id: TeamId,
    pub ruleset: Ruleset,
}

pub struct ChooseRosterCommand {
    pub team_id: TeamId,
    pub roster: Roster,
}