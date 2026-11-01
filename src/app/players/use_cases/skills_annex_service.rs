//! L'annexe des compétences d'une impression (carte 606).
//!
//! Les compétences que portent les joueurs des équipes imprimées — de base,
//! par leur poste, et acquises —, **une seule fois chacune**, avec leur
//! catégorie et leur description lues dans le catalogue.

use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::{
    IPlayerProjectionRepository, ISkillCatalogPort, PlayerProjection,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnexSkill {
    pub name: String,
    pub category_label: String,
    pub description: String,
}

// arch:no-instrument — service de lecture : rassemble les compétences des équipes imprimées
pub async fn collect(
    projections: &dyn IPlayerProjectionRepository,
    catalog: &dyn ISkillCatalogPort,
    team_ids: &[&str],
) -> Vec<AnnexSkill> {
    let mut uids = BTreeSet::new();
    for team_id in team_ids {
        let players = projections
            .find_alive_by_team_id(&TeamId(team_id.to_string()))
            .await
            .unwrap_or_default();
        for p in &players {
            uids.extend(skill_uids(p, catalog));
        }
    }
    resolve(uids, catalog)
}

/// Les compétences de base viennent du poste, comme dans la feuille d'équipe ;
/// les acquises, de la projection.
fn skill_uids(p: &PlayerProjection, catalog: &dyn ISkillCatalogPort) -> Vec<String> {
    let base = catalog
        .find_position(&p.roster_line_id)
        .map(|pos| pos.base_skills)
        .unwrap_or_default();
    base.into_iter()
        .chain(p.acquired_skills.iter().map(|s| s.skill_id.clone()))
        .collect()
}

/// Une compétence inconnue du catalogue est ignorée : sans nom ni description,
/// elle n'aurait rien à imprimer.
fn resolve(uids: BTreeSet<String>, catalog: &dyn ISkillCatalogPort) -> Vec<AnnexSkill> {
    let mut skills: Vec<AnnexSkill> = uids
        .iter()
        .filter_map(|uid| catalog.find_skill(uid))
        .map(|s| AnnexSkill {
            name: s.name,
            category_label: s.category_label,
            description: s.description,
        })
        .collect();
    skills.sort_by_key(|s| s.name.to_lowercase());
    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
    use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;
    use std::sync::Arc;

    fn catalogue() -> SkillCatalogAdapter {
        SkillCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn chaque_competence_une_seule_fois_triee_par_nom() {
        let uids: BTreeSet<String> = ["PAS_DE_COTE", "APPUI_FERME", "PAS_DE_COTE", "INCONNUE"]
            .iter()
            .map(|u| u.to_string())
            .collect();
        let annexe = resolve(uids, &catalogue());
        let noms: Vec<&str> = annexe.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(noms, vec!["Appui Ferme", "Pas de Côté"]);
        assert_eq!(annexe[0].category_label, "Général");
        assert!(annexe[0].description.contains("repoussé"));
    }
}
