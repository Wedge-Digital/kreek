use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, ValueKpo};
use crate::app::players::domain::value_objects::SppCost;
use crate::app::players::ports::ISkillCatalogPort;

#[derive(Debug, Clone, PartialEq)]
pub enum ImprovementCostError {
    SkillNotFound,
    PositionNotFound,
    CategoryNotAccessible,
}

/// Résout le coût réel (SPP) et la valeur d'équipe ajoutée pour l'achat d'une
/// compétence — toujours recalculé serveur, jamais accepté du client.
pub fn resolve_skill_cost(
    catalog: &dyn ISkillCatalogPort,
    roster_line_id: &str,
    skill_id: &str,
    mode: AcquisitionMode,
    level: u8,
) -> Result<(SppCost, ValueKpo), ImprovementCostError> {
    let skill = catalog
        .find_skill(skill_id)
        .ok_or(ImprovementCostError::SkillNotFound)?;
    let access = catalog
        .position_access(roster_line_id)
        .ok_or(ImprovementCostError::PositionNotFound)?;

    let is_secondary = access.secondary_categories.contains(&skill.category);
    let is_primary = access.primary_categories.contains(&skill.category);
    if !is_primary && !is_secondary {
        return Err(ImprovementCostError::CategoryNotAccessible);
    }

    let level_cost = catalog
        .cost_for_level(level, skill.is_elite)
        .expect("niveau plafonné à 6 par next_improvement_level, toujours défini dans la matrice");

    let cost = match mode {
        AcquisitionMode::Chosen => {
            if is_secondary {
                level_cost.chosen_secondary
            } else {
                level_cost.chosen_primary
            }
        }
        AcquisitionMode::Random => level_cost.random,
    };

    let value_delta = catalog.skill_value_delta(is_secondary);

    Ok((
        SppCost::try_new(cost as u8).expect("coût borné par la matrice de référence (<= 99)"),
        ValueKpo(value_delta),
    ))
}

/// Résout le coût réel (SPP) et la valeur d'équipe ajoutée pour une
/// augmentation de caractéristique.
pub fn resolve_stat_cost(
    catalog: &dyn ISkillCatalogPort,
    stat: StatKind,
    level: u8,
) -> (SppCost, ValueKpo) {
    let level_cost = catalog
        .cost_for_level(level, false)
        .expect("niveau plafonné à 6 par next_improvement_level, toujours défini dans la matrice");
    let cost = SppCost::try_new(level_cost.characteristic as u8)
        .expect("coût borné par la matrice de référence (<= 99)");
    (cost, ValueKpo(catalog.stat_value_delta(stat)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
    use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;
    use std::sync::Arc;

    fn catalog() -> SkillCatalogAdapter {
        SkillCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn resolve_skill_cost_primary_access_level_1() {
        let (cost, value) = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "APPUI_FERME",
            AcquisitionMode::Chosen,
            1,
        )
        .unwrap();
        // APPUI_FERME est GENERAL, primary pour la Piétaille, et Standard —
        // niveau 1 : chosen.primary = 6 (le tarif Élite, lui, vaudrait 8)
        assert_eq!(cost.into_inner(), 6);
        assert_eq!(value.0, 20_000);
    }

    #[test]
    fn resolve_skill_cost_secondary_access_costs_more_and_yields_more_value() {
        // STRENGTH est secondary pour la Piétaille — POIGNE_LARGE est STRENGTH/Standard
        let (cost, value) = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "POIGNE_LARGE",
            AcquisitionMode::Chosen,
            1,
        )
        .unwrap();
        assert_eq!(cost.into_inner(), 10);
        assert_eq!(value.0, 40_000);
    }

    #[test]
    fn resolve_skill_cost_category_not_accessible_is_rejected() {
        // PASSING n'est ni primary ([GENERAL]) ni secondary ([STRENGTH]) pour la
        // Piétaille — LANCER_TENDU existe bien, mais dans une catégorie fermée
        let result = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "LANCER_TENDU",
            AcquisitionMode::Chosen,
            1,
        );
        assert_eq!(result, Err(ImprovementCostError::CategoryNotAccessible));
    }

    #[test]
    fn resolve_skill_cost_unknown_skill_is_rejected() {
        let result = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "NOT_A_REAL_SKILL",
            AcquisitionMode::Chosen,
            1,
        );
        assert_eq!(result, Err(ImprovementCostError::SkillNotFound));
    }

    #[test]
    fn resolve_skill_cost_ignores_client_supplied_cost_always_recomputes_from_level() {
        // Même compétence, niveaux différents → coûts différents, jamais un coût "soumis"
        let (cost_lvl1, _) = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "APPUI_FERME",
            AcquisitionMode::Chosen,
            1,
        )
        .unwrap();
        let (cost_lvl3, _) = resolve_skill_cost(
            &catalog(),
            "DEMO_GRANIT__PIETAILLE",
            "APPUI_FERME",
            AcquisitionMode::Chosen,
            3,
        )
        .unwrap();
        assert_ne!(cost_lvl1.into_inner(), cost_lvl3.into_inner());
        assert_eq!(cost_lvl3.into_inner(), 12);
    }

    #[test]
    fn resolve_stat_cost_uses_characteristic_column_regardless_of_stat() {
        let (cost_ma, _) = resolve_stat_cost(&catalog(), StatKind::Ma, 1);
        let (cost_st, _) = resolve_stat_cost(&catalog(), StatKind::St, 1);
        assert_eq!(cost_ma.into_inner(), 14);
        assert_eq!(cost_st.into_inner(), 14);
    }

    #[test]
    fn resolve_stat_cost_value_delta_depends_on_stat() {
        let (_, value_ma) = resolve_stat_cost(&catalog(), StatKind::Ma, 1);
        let (_, value_st) = resolve_stat_cost(&catalog(), StatKind::St, 1);
        assert_eq!(value_ma.0, 20_000);
        assert_eq!(value_st.0, 60_000);
    }
}
