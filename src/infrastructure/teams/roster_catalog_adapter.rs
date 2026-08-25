use crate::app::references::domain::port::IReferenceRepository;
use crate::app::teams::ports::{
    CatalogPositionDto, CrossLimitDto, IRosterCatalogPort, RosterCatalogDto, SkillBadgeDto,
    StaffPriceDto,
};
use std::sync::Arc;

pub struct RosterCatalogAdapter {
    refs: Arc<dyn IReferenceRepository>,
}

impl RosterCatalogAdapter {
    pub fn new(refs: Arc<dyn IReferenceRepository>) -> Self {
        Self { refs }
    }

    /// Traduit les uids de compétences en badges affichables. Un uid absent du
    /// corpus est rendu tel quel : mieux vaut « BLOODLUST_2 » à l'écran qu'une
    /// compétence escamotée, qui ferait croire au coach que le joueur ne l'a
    /// pas.
    fn skill_badges(&self, uids: &[String]) -> Vec<SkillBadgeDto> {
        uids.iter()
            .map(|uid| match self.refs.find_skill_by_uid(uid) {
                Some(s) => SkillBadgeDto {
                    name: s.name.clone(),
                    category: s.category.clone(),
                },
                None => SkillBadgeDto {
                    name: uid.clone(),
                    category: String::new(),
                },
            })
            .collect()
    }

    fn staff_prices(&self) -> Vec<StaffPriceDto> {
        self.refs
            .list_staff()
            .iter()
            .map(|s| StaffPriceDto {
                uid: s.uid.clone(),
                name: s.name.clone(),
                price: s.price,
                max_quantity: s.max_quantity,
            })
            .collect()
    }
}

/// « Lineman a vil prix » — la règle qui annule le prix de base des linemen
/// dans la valeur d'équipe.
///
/// L'uid reste ici : `teams` lit une règle, pas un identifiant de corpus qu'il
/// faudrait aller comprendre ailleurs. Une constante nommée plutôt qu'un
/// littéral au milieu d'un `any()`, pour qu'une recherche sur ce nom trouve
/// aussi son sens.
const LOW_COST_LINEMEN: &str = "LOW_COST_LINEMEN";

impl IRosterCatalogPort for RosterCatalogAdapter {
    fn find_catalog(&self, roster_id: &str) -> Option<RosterCatalogDto> {
        let team = self.refs.find_team_by_uid(roster_id)?;
        Some(RosterCatalogDto {
            logo: team.logo.clone(),
            linemen_are_free: team.special_rules.iter().any(|r| r == LOW_COST_LINEMEN),
            reroll_base_cost: team.reroll_cost,
            positions: team
                .available_players
                .iter()
                .map(|p| CatalogPositionDto {
                    uid: p.uid.clone(),
                    position_name: p.position_name.clone(),
                    cost: p.cost,
                    max_quantity: p.max_quantity,
                    is_journeyman: p.is_journeyman,
                    ma: p.ma,
                    st: p.st,
                    ag: p.ag,
                    pa: p.pa,
                    av: p.av,
                    skills: self.skill_badges(&p.skills),
                })
                .collect(),
            cross_limits: team
                .cross_limit
                .iter()
                .map(|cl| CrossLimitDto {
                    max: cl.max,
                    position_uids: cl.position_uids.clone(),
                })
                .collect(),
            allowed_staff: team.allowed_staff.clone(),
            staff_prices: self.staff_prices(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> RosterCatalogAdapter {
        RosterCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn les_prix_de_staff_viennent_des_donnees_de_reference() {
        let c = adapter().find_catalog("DEMO_GRANIT").unwrap();
        assert_eq!(c.staff_price("APOTHECARY"), 50);
        assert_eq!(c.staff_price("COACH_ASSISTANTS"), 10);
        assert_eq!(c.staff_price("CHEERLEADERS"), 10);
    }

    #[test]
    fn une_ligne_de_staff_absente_vaut_zero_plutot_que_de_faire_echouer() {
        assert_eq!(
            adapter()
                .find_catalog("DEMO_GRANIT")
                .unwrap()
                .staff_price("X"),
            0
        );
    }

    /// Le champ que la carte 258 fait enfin remonter jusqu'ici.
    #[test]
    fn les_limites_croisees_du_roster_sont_exposees() {
        let c = adapter().find_catalog("DEMO_GRANIT").unwrap();
        assert_eq!(c.cross_limits.len(), 1);
        assert_eq!(c.cross_limits[0].max, 2);
        assert!(c.cross_limits[0]
            .position_uids
            .contains(&"DEMO_GRANIT__COLOSSE".to_string()));
    }

    #[test]
    fn un_roster_sans_limite_croisee_en_expose_zero() {
        assert!(adapter()
            .find_catalog("DEMO_ZEPHYR")
            .unwrap()
            .cross_limits
            .is_empty());
    }

    #[test]
    fn les_postes_portent_le_marqueur_de_journalier() {
        let c = adapter().find_catalog("DEMO_GRANIT").unwrap();
        let journaliers: Vec<_> = c.positions.iter().filter(|p| p.is_journeyman).collect();
        assert_eq!(journaliers.len(), 1, "exactement un journalier par roster");
        assert_eq!(journaliers[0].uid, "DEMO_GRANIT__PIETAILLE");
    }

    #[test]
    fn un_roster_inconnu_ne_donne_aucun_catalogue() {
        assert!(adapter().find_catalog("INEXISTANT").is_none());
    }
}
