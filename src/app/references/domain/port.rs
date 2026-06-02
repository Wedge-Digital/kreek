use crate::app::references::domain::models::{
    Inducement, League, Skill, SkillCategory, SpecialRule, Staff, StarPlayer, Team,
};

pub trait IReferenceRepository: Send + Sync {
    fn list_inducements(&self) -> &[Inducement];
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
}
