use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::ports::{
    IReferenceDataPort, PlayerPositionDefinition, RosterDefinition, SkillCostResult,
    SkillDefinition, StaffDefinition,
};
use std::sync::Arc;

pub struct ReferenceDataAdapter {
    repo: Arc<dyn IReferenceRepository>,
}

impl ReferenceDataAdapter {
    pub fn new(repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { repo }
    }
}

impl IReferenceDataPort for ReferenceDataAdapter {
    fn find_roster_definition(&self, roster_uid: &str) -> Option<RosterDefinition> {
        let team = self.repo.find_team_by_uid(roster_uid)?;
        Some(RosterDefinition {
            uid: team.uid.clone(),
            name: team.name.clone(),
            reroll_cost: team.reroll_cost,
            available_players: team
                .available_players
                .iter()
                .map(|p| {
                    let skills = p
                        .skills
                        .iter()
                        .map(|uid| {
                            let name = self
                                .repo
                                .find_skill_by_uid(uid)
                                .map(|s| s.name.clone())
                                .unwrap_or_else(|| uid.clone());
                            SkillDefinition { uid: uid.clone(), name }
                        })
                        .collect();
                    PlayerPositionDefinition {
                        uid: p.uid.clone(),
                        position_name: p.position_name.clone(),
                        cost: p.cost,
                        max_quantity: p.max_quantity,
                        ma: p.ma,
                        st: p.st,
                        ag: p.ag,
                        pa: p.pa,
                        av: p.av,
                        skills,
                    }
                })
                .collect(),
            allowed_staff_uids: team.allowed_staff.clone(),
            leagues: team.leagues.clone(),
            special_rules: team.special_rules.clone(),
        })
    }

    fn resolve_skill_cost(
        &self,
        roster_line_id: &str,
        skill_id: &str,
        mode: &str,
    ) -> Option<SkillCostResult> {
        let position = self.repo.find_position_by_uid(roster_line_id)?;
        let skill = self.repo.find_skill_by_uid(skill_id)?;

        let is_primary = position.primary_access.iter().any(|c| c == &skill.category);
        let is_elite = skill.skill_type == "Élite";

        let pricing = self.repo.skill_cost_matrix().iter().find(|l| l.level == 1)?;

        let spp_cost = match mode {
            "random" => pricing.random_for(is_elite),
            _ => {
                let costs = pricing.chosen_for(is_elite);
                if is_primary { costs.primary } else { costs.secondary }
            }
        };

        Some(SkillCostResult { spp_cost })
    }

    fn list_staff_definitions(&self) -> Vec<StaffDefinition> {
        self.repo
            .list_staff()
            .iter()
            .map(|s| StaffDefinition {
                uid: s.uid.clone(),
                name: s.name.clone(),
                price: s.price,
                max_quantity: s.max_quantity as u8,
            })
            .collect()
    }

}
