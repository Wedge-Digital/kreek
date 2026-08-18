use crate::app::shared_kernel::bloodbowl::staff::StaffId;
use crate::app::shared_kernel::bloodbowl::team::{TeamId, TeamName};
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, UserId};
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::roster::{PlayerId, Roster};
use crate::app::team_creation::domain::ruleset::Ruleset;

#[derive(Debug)]
pub struct RegisterNewTeamCommand {
    pub team_name: TeamName,
    pub coach_id: CoachId,
    pub logo_url: Option<CloudinaryImage>,
    pub created_by: UserId,
    pub competition_id: String,
    pub season_id: String,
    pub creation_rules: CreationRules,
}

#[derive(Debug)]
pub struct SelectRulesetCommand {
    pub team_id: TeamId,
    pub ruleset: Ruleset,
}

#[derive(Debug)]
pub struct ChooseRosterCommand {
    pub team_id: TeamId,
    pub roster: Roster,
}

#[derive(Debug)]
pub struct HirePlayerCommand {
    pub team_id: TeamId,
    pub space_id: String,
    pub player_id: PlayerId,
}

#[derive(Debug)]
pub struct FirePlayerCommand {
    pub team_id: TeamId,
    pub space_id: String,
    pub player_id: PlayerId,
}

#[derive(Debug)]
pub struct BuyStaffCommand {
    pub team_id: TeamId,
    pub space_id: String,
    pub staff_id: StaffId,
}

#[derive(Debug)]
pub struct RemoveStaffCommand {
    pub team_id: TeamId,
    pub space_id: String,
    pub staff_id: StaffId,
}

#[derive(Debug)]
pub struct BuyRerollCommand {
    pub team_id: TeamId,
    pub space_id: String,
}

#[derive(Debug)]
pub struct RemoveRerollCommand {
    pub team_id: TeamId,
    pub space_id: String,
}

#[derive(Debug)]
pub struct SubmitTeamCommand {
    pub team_id: TeamId,
    pub space_id: String,
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub season_name: String,
    pub coach_name: String,
    pub auto_enroll: bool,
}
