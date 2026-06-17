use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::ports::{
    IReferenceDataPort, PlayerPositionDefinition, RosterDefinition, SkillDefinition,
    StaffDefinition,
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
