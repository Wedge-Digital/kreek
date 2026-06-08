use crate::app::references::domain::models::{
    Inducement, League, PlayerPosition, Skill, SkillCategory, SkillCostLevel, SpecialRule, Staff,
    StarPlayer, Team,
};
use crate::app::references::domain::port::IReferenceRepository;
use serde::Deserialize;
use crate::app::shared_kernel::RosterDefinition::RosterDefinition;
use crate::app::team_creation::domain::roster::{RosterId, RosterName};
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
            name: RosterName(team.name.clone()),
        }).collect();
        rosters.sort_unstable_by(|a, b| a.id.0.cmp(&b.id.0));
        rosters
    }

    fn list_inducements(&self) -> &[Inducement] {
        &self.inducements
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
}
