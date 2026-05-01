use crate::app::team_creation::common_types::{CloudinaryImage, CoachId, TeamId, TeamName, UserId};
use crate::app::team_creation::roster::Roster;
use crate::app::team_creation::ruleset::Ruleset;

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