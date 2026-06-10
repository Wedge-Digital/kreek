use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff,
    StarPlayer, Team,
};
use crate::app::shared_kernel::inducement_definition::InducementDefinition;
use crate::app::shared_kernel::roster_definition::RosterDefinition;

pub trait IReferenceRepository: Send + Sync {
    fn list_roster_definitions(&self) -> Vec<RosterDefinition>;
    fn list_inducements(&self) -> Vec<InducementDefinition>;
    fn list_star_players(&self) -> &[StarPlayer];
    fn list_teams(&self) -> &[Team];
    fn list_skills(&self) -> &[Skill];
    fn list_skill_categories(&self) -> &[SkillCategory];
    fn list_special_rules(&self) -> &[SpecialRule];
    fn list_staff(&self) -> &[Staff];
    fn list_leagues(&self) -> &[League];

    fn find_inducement_by_uid(&self, uid: &str) -> Option<&Inducement>;
    fn find_star_player_by_uid(&self, uid: &str) -> Option<&StarPlayer>;
    fn find_team_by_uid(&self, uid: &str) -> Option<&Team>;
    fn find_skill_by_uid(&self, uid: &str) -> Option<&Skill>;
    fn find_position_by_uid(&self, uid: &str) -> Option<&PlayerPosition>;
    fn skill_cost_matrix(&self) -> &[SkillCostLevel];
}
