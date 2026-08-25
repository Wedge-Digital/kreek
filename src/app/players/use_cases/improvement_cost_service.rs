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
        // Une customisation ne passe jamais par ce service : elle ne coûte rien
        // et n'ajoute aucune valeur, c'est sa définition. Zéro plutôt qu'un
        // `unreachable!` — ce chemin sert aussi à recalculer le coût d'une
        // compétence déjà acquise pour l'afficher, et un panic y serait
        // disproportionné.
        AcquisitionMode::Customised => 0,
    };

    // `skill.is_elite`, comme le coût en SPP juste au-dessus : la valeur et
    // le coût lisent le même drapeau, il n'y a pas deux notions d'élitisme.
    let value_delta = catalog.skill_value_delta(is_secondary, skill.is_elite);

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
    use crate::app::players::io::app_events::team_created_listener::initial_skill_value_delta;
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
        assert_eq!(value.0, 20);
    }

    /// Vrai pour ce poste si la catégorie de la compétence est dans son accès
    /// primaire — c'est ce que le listener de création calcule à partir de la
    /// ligne de roster, et non une donnée de la compétence seule.
    fn is_primary_for(
        catalog: &dyn ISkillCatalogPort,
        roster_line_id: &str,
        skill_id: &str,
    ) -> bool {
        let skill = catalog.find_skill(skill_id).unwrap();
        catalog
            .position_access(roster_line_id)
            .unwrap()
            .primary_categories
            .contains(&skill.category)
    }

    /// **La valeur d'une compétence dépend du couple (compétence, poste)**, pas
    /// de la compétence seule : une même catégorie peut être primaire pour un
    /// poste et secondaire pour un autre.
    ///
    /// Ici STRENGTH est primaire pour le Percuteur (`P=[GENERAL, STRENGTH]`) et
    /// secondaire pour la Piétaille (`S=[STRENGTH]`) — deux postes de la même
    /// équipe. POIGNE_LARGE, compétence STRENGTH, vaut donc 20 kPo sur l'un et
    /// 40 kPo sur l'autre.
    #[test]
    fn la_valeur_d_une_competence_depend_du_poste_qui_l_acquiert() {
        let catalog = catalog();

        let (_, sur_percuteur) = resolve_skill_cost(
            &catalog,
            "DEMO_GRANIT__PERCUTEUR",
            "POIGNE_LARGE",
            AcquisitionMode::Chosen,
            1,
        )
        .unwrap();
        let (_, sur_pietaille) = resolve_skill_cost(
            &catalog,
            "DEMO_GRANIT__PIETAILLE",
            "POIGNE_LARGE",
            AcquisitionMode::Chosen,
            1,
        )
        .unwrap();

        assert_eq!(
            sur_percuteur.0, 20,
            "STRENGTH est primaire pour le Percuteur"
        );
        assert_eq!(
            sur_pietaille.0, 40,
            "STRENGTH est secondaire pour la Piétaille"
        );
        assert_ne!(
            sur_percuteur, sur_pietaille,
            "la valeur ne doit pas dépendre de la seule compétence"
        );
    }

    /// Le barème de la carte 387 : dix de plus pour une Élite, dans les deux
    /// accès. Les quatre valeurs viennent du corpus, aucune n'est dans le code.
    #[test]
    fn une_elite_vaut_dix_kpo_de_plus_qu_une_standard_de_meme_acces() {
        let catalog = catalog();
        let valeur = |skill_id: &str| {
            let is_primary = is_primary_for(&catalog, "DEMO_GRANIT__PIETAILLE", skill_id);
            let is_elite = catalog.find_skill(skill_id).unwrap().is_elite;
            initial_skill_value_delta(&catalog, is_primary, is_elite)
        };

        assert_eq!(
            valeur("APPUI_FERME"),
            ValueKpo(20),
            "GENERAL primaire, Standard"
        );
        assert_eq!(
            valeur("SECOND_SOUFFLE"),
            ValueKpo(30),
            "GENERAL primaire, Élite"
        );
        assert_eq!(
            valeur("POIGNE_LARGE"),
            ValueKpo(40),
            "STRENGTH secondaire, Standard"
        );
        assert_eq!(
            valeur("MASSE_INEBRANLABLE"),
            ValueKpo(50),
            "STRENGTH secondaire, Élite"
        );
    }

    /// Le mode d'acquisition ne change pas la valeur — comme le coût en SPP,
    /// déjà réglé par `random_for(is_elite)`. Une Élite tirée au hasard vaut
    /// autant qu'une Élite choisie.
    #[test]
    fn le_mode_d_acquisition_ne_change_pas_la_valeur_d_une_elite() {
        let catalog = catalog();
        let par_mode = |mode| {
            resolve_skill_cost(
                &catalog,
                "DEMO_GRANIT__PIETAILLE",
                "SECOND_SOUFFLE",
                mode,
                1,
            )
            .unwrap()
            .1
        };

        assert_eq!(par_mode(AcquisitionMode::Chosen), ValueKpo(30));
        assert_eq!(par_mode(AcquisitionMode::Random), ValueKpo(30));
    }

    /// Le cœur de la carte 249 : avant elle, la même compétence sur le même
    /// joueur valait 20 kPo (ou 30 si élite) obtenue à la création et 20 000 à
    /// l'achat en SPP — deux barèmes, deux unités. « Origine » désigne ici le
    /// mode d'acquisition, pas le poste : la dépendance au poste, elle, est
    /// légitime et couverte par le test ci-dessus.
    ///
    /// L'accès primaire est dérivé du poste, jamais codé en dur — sinon le test
    /// passerait encore si la résolution d'accès était cassée.
    #[test]
    fn une_competence_vaut_le_meme_prix_a_la_creation_et_a_l_achat_en_spp() {
        let catalog = catalog();

        for (roster_line_id, skill_id) in [
            ("DEMO_GRANIT__PIETAILLE", "APPUI_FERME"), // GENERAL, primaire
            ("DEMO_GRANIT__PIETAILLE", "POIGNE_LARGE"), // STRENGTH, secondaire
            ("DEMO_GRANIT__PERCUTEUR", "POIGNE_LARGE"), // STRENGTH, primaire ici
            // La parité doit tenir aussi sous le barème Élite, sans quoi une
            // Élite obtenue à la création vaudrait dix de moins qu'achetée.
            ("DEMO_GRANIT__PIETAILLE", "SECOND_SOUFFLE"), // GENERAL, primaire, Élite
            ("DEMO_GRANIT__PIETAILLE", "MASSE_INEBRANLABLE"), // STRENGTH, secondaire, Élite
        ] {
            let (_, achat) = resolve_skill_cost(
                &catalog,
                roster_line_id,
                skill_id,
                AcquisitionMode::Chosen,
                1,
            )
            .unwrap();
            let is_primary = is_primary_for(&catalog, roster_line_id, skill_id);
            let is_elite = catalog.find_skill(skill_id).unwrap().is_elite;
            let creation = initial_skill_value_delta(&catalog, is_primary, is_elite);

            assert_eq!(
                creation, achat,
                "{skill_id} sur {roster_line_id} ne vaut pas le même prix selon son mode d'acquisition"
            );
        }
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
        assert_eq!(value.0, 40);
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
        assert_eq!(value_ma.0, 20);
        assert_eq!(value_st.0, 60);
    }
}
