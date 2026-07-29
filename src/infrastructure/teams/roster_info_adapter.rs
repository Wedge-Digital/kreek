use crate::app::references::domain::port::IReferenceRepository;
use crate::app::teams::ports::{IRosterInfoPort, RosterInfoDto};
use std::sync::Arc;

pub struct RosterInfoAdapter {
    refs: Arc<dyn IReferenceRepository>,
}

impl RosterInfoAdapter {
    pub fn new(refs: Arc<dyn IReferenceRepository>) -> Self {
        Self { refs }
    }

    /// Prix d'une ligne de staff, par identifiant de référence. Zéro si la
    /// ligne est absente du corpus : mieux vaut une TV incomplète qu'un
    /// démarrage impossible sur un jeu de données partiel.
    fn staff_price(&self, uid: &str) -> u32 {
        self.refs
            .list_staff()
            .iter()
            .find(|s| s.uid == uid)
            .map(|s| s.price)
            .unwrap_or(0)
    }
}

impl IRosterInfoPort for RosterInfoAdapter {
    fn find_roster_info(&self, roster_id: &str) -> Option<RosterInfoDto> {
        let team = self.refs.find_team_by_uid(roster_id)?;
        Some(RosterInfoDto {
            logo: team.logo.clone(),
            reroll_cost: team.reroll_cost,
            apothecary_price: self.staff_price("APOTHECARY"),
            assistant_price: self.staff_price("COACH_ASSISTANTS"),
            cheerleader_price: self.staff_price("CHEERLEADERS"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> RosterInfoAdapter {
        RosterInfoAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn les_prix_de_staff_viennent_des_donnees_de_reference() {
        let info = adapter().find_roster_info("DEMO_GRANIT").unwrap();
        assert_eq!(info.apothecary_price, 50);
        assert_eq!(info.assistant_price, 10);
        assert_eq!(info.cheerleader_price, 10);
    }

    #[test]
    fn une_ligne_de_staff_absente_vaut_zero_plutot_que_de_faire_echouer() {
        assert_eq!(adapter().staff_price("STAFF_INEXISTANT"), 0);
    }
}
