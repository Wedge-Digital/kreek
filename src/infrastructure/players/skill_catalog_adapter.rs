use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::ports::{
    ISkillCatalogPort, PositionAccessDto, PositionCatalogEntryDto, SkillCatalogEntryDto,
    SkillCostLevelDto,
};
use crate::app::references::domain::port::IReferenceRepository;
use std::sync::Arc;

pub struct SkillCatalogAdapter {
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl SkillCatalogAdapter {
    pub fn new(reference_repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { reference_repo }
    }
}

impl ISkillCatalogPort for SkillCatalogAdapter {
    fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto> {
        let skill = self.reference_repo.find_skill_by_uid(skill_id)?;
        Some(SkillCatalogEntryDto {
            skill_id: skill.uid.clone(),
            name: skill.name.clone(),
            category: skill.category.clone(),
            is_elite: skill.skill_type == "Élite",
        })
    }

    fn find_position(&self, roster_line_id: &str) -> Option<PositionCatalogEntryDto> {
        let position = self.reference_repo.find_position_by_uid(roster_line_id)?;
        Some(PositionCatalogEntryDto {
            position_name: position.position_name.clone(),
            cost: position.cost,
            ma: position.ma,
            st: position.st,
            ag: position.ag,
            pa: position.pa,
            av: position.av,
            base_skills: position.skills.clone(),
            primary_categories: position.primary_access.clone(),
            secondary_categories: position.secondary_access.clone(),
        })
    }

    fn position_access(&self, roster_line_id: &str) -> Option<PositionAccessDto> {
        let position = self.reference_repo.find_position_by_uid(roster_line_id)?;
        Some(PositionAccessDto {
            primary_categories: position.primary_access.clone(),
            secondary_categories: position.secondary_access.clone(),
        })
    }

    fn cost_for_level(&self, level: u8, is_elite: bool) -> Option<SkillCostLevelDto> {
        let cost = self
            .reference_repo
            .skill_cost_matrix()
            .iter()
            .find(|l| l.level == level)?;
        let chosen = cost.chosen_for(is_elite);
        Some(SkillCostLevelDto {
            level: cost.level,
            chosen_primary: chosen.primary as u32,
            chosen_secondary: chosen.secondary as u32,
            random: cost.random_for(is_elite) as u32,
            characteristic: cost.characteristic as u32,
        })
    }

    fn skill_value_delta(&self, is_secondary_access: bool) -> u32 {
        self.reference_repo
            .improvement_skill_value_delta(is_secondary_access)
    }

    fn stat_value_delta(&self, stat: StatKind) -> u32 {
        match stat {
            StatKind::Ma => self.reference_repo.improvement_stat_value_delta_ma(),
            StatKind::St => self.reference_repo.improvement_stat_value_delta_st(),
            StatKind::Ag => self.reference_repo.improvement_stat_value_delta_ag(),
            StatKind::Pa => self.reference_repo.improvement_stat_value_delta_pa(),
            StatKind::Av => self.reference_repo.improvement_stat_value_delta_av(),
        }
    }

    fn touchdown_spp(&self) -> u8 {
        self.reference_repo.touchdown_spp()
    }
    fn pass_spp(&self) -> u8 {
        self.reference_repo.pass_spp()
    }
    fn interception_spp(&self) -> u8 {
        self.reference_repo.interception_spp()
    }
    fn casualty_spp(&self) -> u8 {
        self.reference_repo.casualty_spp()
    }
    fn mvp_spp(&self) -> u8 {
        self.reference_repo.mvp_spp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> SkillCatalogAdapter {
        SkillCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn find_skill_maps_elite_flag_correctly() {
        let skill = adapter()
            .find_skill("SECOND_SOUFFLE")
            .expect("SECOND_SOUFFLE doit exister dans le référentiel");
        assert_eq!(skill.name, "Second Souffle");
        assert_eq!(skill.category, "GENERAL");
        assert!(skill.is_elite);
    }

    #[test]
    fn find_skill_unknown_uid_returns_none() {
        assert!(adapter().find_skill("NOT_A_REAL_SKILL").is_none());
    }

    #[test]
    fn position_access_resolves_primary_and_secondary() {
        let access = adapter()
            .position_access("DEMO_GRANIT__PIETAILLE")
            .expect("position doit exister");
        assert_eq!(access.primary_categories, vec!["GENERAL"]);
        assert!(access
            .secondary_categories
            .contains(&"STRENGTH".to_string()));
    }

    #[test]
    fn position_access_unknown_uid_returns_none() {
        assert!(adapter().position_access("NOT_A_REAL_POSITION").is_none());
    }

    #[test]
    fn cost_for_level_out_of_bounds_returns_none() {
        assert!(adapter().cost_for_level(0, false).is_none());
        assert!(adapter().cost_for_level(200, false).is_none());
    }

    #[test]
    fn cost_for_level_matches_official_matrix_level_1() {
        let cost = adapter()
            .cost_for_level(1, false)
            .expect("niveau 1 doit exister");
        assert_eq!(cost.chosen_primary, 6);
        assert_eq!(cost.chosen_secondary, 10);
        assert_eq!(cost.random, 3);
        assert_eq!(cost.characteristic, 14);
    }

    #[test]
    fn stat_value_delta_matches_official_table() {
        let a = adapter();
        assert_eq!(a.stat_value_delta(StatKind::Ma), 20_000);
        assert_eq!(a.stat_value_delta(StatKind::St), 60_000);
        assert_eq!(a.stat_value_delta(StatKind::Ag), 30_000);
        assert_eq!(a.stat_value_delta(StatKind::Pa), 20_000);
        assert_eq!(a.stat_value_delta(StatKind::Av), 10_000);
    }
}
