use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, UserId};
use crate::app::shared_kernel::team::{TeamId, TeamName};
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::roster::{PlayerId, Roster};
use crate::app::team_creation::domain::ruleset::Ruleset;

pub struct RegisterNewTeamCommand {
    pub team_name:      TeamName,
    pub coach_id:       CoachId,
    pub logo_url:       Option<CloudinaryImage>,
    pub created_by:     UserId,
    pub competition_id: String,
    pub season_id:      String,
    pub creation_rules: CreationRules,
}

pub struct SelectRulesetCommand {
    pub team_id: TeamId,
    pub ruleset: Ruleset,
}

pub struct ChooseRosterCommand {
    pub team_id: TeamId,
    pub roster:  Roster,
}

pub struct HirePlayerCommand {
    pub team_id:   TeamId,
    pub space_id:  String,
    pub player_id: PlayerId,
}