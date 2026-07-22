use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff,
    StarPlayer, Team,
};
use crate::app::references::domain::port::IReferenceRepository;
use serde::Deserialize;
use crate::app::shared_kernel::inducement_definition::{InducementCost, InducementDefinition, InducementId, InducementName};
use crate::app::shared_kernel::roster_definition::RosterDefinition;
use crate::app::shared_kernel::common_types::RosterId;
// ── Raw JSON wrappers (deserialization only) ──────────────────────────────────

#[derive(Deserialize)]
struct InducementsFile {
    inducements: Vec<Inducement>,
}

#[derive(Deserialize)]
struct StarPlayersFile {
    star_players: Vec<StarPlayer>,
}

#[derive(Deserialize)]
struct TeamsFile {
    teams: Vec<Team>,
}

#[derive(Deserialize)]
struct SkillsFile {
    skills: Vec<Skill>,
}

#[derive(Deserialize)]
struct SkillCatFile {
    skill_categories: Vec<SkillCategory>,
}

#[derive(Deserialize)]
struct SpecialRulesFile {
    special_rules: Vec<SpecialRule>,
}

#[derive(Deserialize)]
struct StaffFile {
    staff: Vec<Staff>,
}

#[derive(Deserialize)]
struct LeaguesFile {
    leagues: Vec<League>,
}

// skill_cost.json est un tableau JSON de premier niveau, pas d'objet wrapper
type SkillCostFile = Vec<SkillCostLevel>;

// ── In-memory repository ──────────────────────────────────────────────────────

pub struct InMemoryReferenceRepository {
    inducements:       Vec<Inducement>,
    star_players:      Vec<StarPlayer>,
    teams:             Vec<Team>,
    skills:            Vec<Skill>,
    skill_categories:  Vec<SkillCategory>,
    special_rules:     Vec<SpecialRule>,
    staff:             Vec<Staff>,
    leagues:           Vec<League>,
    skill_cost_matrix: Vec<SkillCostLevel>,
}

impl InMemoryReferenceRepository {
    pub fn load() -> Self {
        Self {
            inducements: serde_json::from_str::<InducementsFile>(include_str!(
                "../../../../../assets/references/inducements_fr.json"
            ))
            .expect("inducements_fr.json invalid")
            .inducements,

            star_players: serde_json::from_str::<StarPlayersFile>(include_str!(
                "../../../../../assets/references/star_players_fr.json"
            ))
            .expect("star_players_fr.json invalid")
            .star_players,

            teams: serde_json::from_str::<TeamsFile>(include_str!(
                "../../../../../assets/references/teams_fr.json"
            ))
            .expect("teams_fr.json invalid")
            .teams,

            skills: serde_json::from_str::<SkillsFile>(include_str!(
                "../../../../../assets/references/skills_fr.json"
            ))
            .expect("skills_fr.json invalid")
            .skills,

            skill_categories: serde_json::from_str::<SkillCatFile>(include_str!(
                "../../../../../assets/references/skill_cat_fr.json"
            ))
            .expect("skill_cat_fr.json invalid")
            .skill_categories,

            special_rules: serde_json::from_str::<SpecialRulesFile>(include_str!(
                "../../../../../assets/references/special_rules_fr.json"
            ))
            .expect("special_rules_fr.json invalid")
            .special_rules,

            staff: serde_json::from_str::<StaffFile>(include_str!(
                "../../../../../assets/references/staff_fr.json"
            ))
            .expect("staff_fr.json invalid")
            .staff,

            leagues: serde_json::from_str::<LeaguesFile>(include_str!(
                "../../../../../assets/references/leagues_fr.json"
            ))
            .expect("leagues_fr.json invalid")
            .leagues,

            skill_cost_matrix: serde_json::from_str::<SkillCostFile>(include_str!(
                "../../../../../assets/references/skill_cost.json"
            ))
            .expect("skill_cost.json invalid"),
        }
    }
}

impl IReferenceRepository for InMemoryReferenceRepository {
    fn list_roster_definitions(&self) -> Vec<RosterDefinition> {
        let mut rosters: Vec<RosterDefinition> = self.teams.iter().map(|team: &Team| RosterDefinition {
            id: RosterId(team.uid.clone()),
            name: team.name.clone(),
        }).collect();
        rosters.sort_unstable_by(|a, b| a.id.0.cmp(&b.id.0));
        rosters
    }

    fn list_inducements(&self) -> Vec<InducementDefinition>{
        let mut inducements: Vec<InducementDefinition> = self.inducements.iter().map(|inducement| InducementDefinition {
            id: InducementId(inducement.uid.clone()),
            cost: InducementCost::try_new(inducement.cost).expect("invalid inducement cost"),
            name: InducementName(inducement.name.clone()),
        }).collect();
        inducements.sort_unstable_by(|a, b| a.id.0.cmp(&b.id.0));
        inducements
    }

    fn list_star_players(&self) -> &[StarPlayer] {
        &self.star_players
    }
    fn list_teams(&self) -> &[Team] {
        &self.teams
    }
    fn list_skills(&self) -> &[Skill] {
        &self.skills
    }
    fn list_skill_categories(&self) -> &[SkillCategory] {
        &self.skill_categories
    }
    fn list_special_rules(&self) -> &[SpecialRule] {
        &self.special_rules
    }
    fn list_staff(&self) -> &[Staff] {
        &self.staff
    }
    fn list_leagues(&self) -> &[League] {
        &self.leagues
    }

    fn find_inducement_by_uid(&self, uid: &str) -> Option<&Inducement> {
        self.inducements.iter().find(|x| x.uid == uid)
    }
    fn find_star_player_by_uid(&self, uid: &str) -> Option<&StarPlayer> {
        self.star_players.iter().find(|x| x.uid == uid)
    }
    fn find_team_by_uid(&self, uid: &str) -> Option<&Team> {
        self.teams.iter().find(|x| x.uid == uid)
    }
    fn find_skill_by_uid(&self, uid: &str) -> Option<&Skill> {
        self.skills.iter().find(|x| x.uid == uid)
    }
    fn find_position_by_uid(&self, uid: &str) -> Option<&PlayerPosition> {
        self.teams
            .iter()
            .flat_map(|t| t.available_players.iter())
            .find(|p| p.uid == uid)
    }

    fn skill_cost_matrix(&self) -> &[SkillCostLevel] {
        &self.skill_cost_matrix
    }

    fn touchdown_spp(&self) -> u8 { 3 }
    fn pass_spp(&self) -> u8 { 1 }
    fn interception_spp(&self) -> u8 { 2 }
    fn casualty_spp(&self) -> u8 { 2 }
    fn mvp_spp(&self) -> u8 { 4 }

    fn improvement_skill_value_delta(&self, is_secondary_access: bool) -> u32 {
        if is_secondary_access { 40_000 } else { 20_000 }
    }
    fn improvement_stat_value_delta_ma(&self) -> u32 { 20_000 }
    fn improvement_stat_value_delta_st(&self) -> u32 { 60_000 }
    fn improvement_stat_value_delta_ag(&self) -> u32 { 30_000 }
    fn improvement_stat_value_delta_pa(&self) -> u32 { 20_000 }
    fn improvement_stat_value_delta_av(&self) -> u32 { 10_000 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spp_scale_matches_blood_bowl_standard_barème() {
        let repo = InMemoryReferenceRepository::load();
        assert_eq!(repo.touchdown_spp(), 3);
        assert_eq!(repo.pass_spp(), 1);
        assert_eq!(repo.interception_spp(), 2);
        assert_eq!(repo.casualty_spp(), 2);
        assert_eq!(repo.mvp_spp(), 4);
    }

    #[test]
    fn improvement_value_delta_matches_official_table() {
        let repo = InMemoryReferenceRepository::load();
        assert_eq!(repo.improvement_skill_value_delta(false), 20_000);
        assert_eq!(repo.improvement_skill_value_delta(true), 40_000);
        assert_eq!(repo.improvement_stat_value_delta_ma(), 20_000);
        assert_eq!(repo.improvement_stat_value_delta_st(), 60_000);
        assert_eq!(repo.improvement_stat_value_delta_ag(), 30_000);
        assert_eq!(repo.improvement_stat_value_delta_pa(), 20_000);
        assert_eq!(repo.improvement_stat_value_delta_av(), 10_000);
    }
}
