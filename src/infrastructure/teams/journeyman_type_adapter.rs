use crate::app::references::domain::port::IReferenceRepository;
use crate::app::teams::ports::{IJourneymanTypePort, JourneymanTypeDto};
use std::sync::Arc;

pub struct JourneymanTypeAdapter {
    refs: Arc<dyn IReferenceRepository>,
}

impl JourneymanTypeAdapter {
    pub fn new(refs: Arc<dyn IReferenceRepository>) -> Self {
        Self { refs }
    }
}

fn fallback() -> JourneymanTypeDto {
    JourneymanTypeDto {
        position_name: "Lineman".to_string(),
        price_kpo: 0,
    }
}

impl IJourneymanTypePort for JourneymanTypeAdapter {
    /// Le journalier est désigné par le champ explicite `is_journeyman` du
    /// corpus, exactement un par roster — et non plus par le poste au
    /// `max_quantity` le plus élevé, une heuristique qui tombait juste par
    /// coïncidence. `match_report` lit déjà ce champ (`ref_team_data_adapter`) ;
    /// les deux BCs s'accordent désormais sur la même règle.
    fn journeyman_type_for_roster(&self, roster_id: &str) -> JourneymanTypeDto {
        let Some(team) = self.refs.find_team_by_uid(roster_id) else {
            return fallback();
        };

        team.available_players
            .iter()
            .find(|p| p.is_journeyman)
            .map(|p| JourneymanTypeDto {
                position_name: p.position_name.clone(),
                price_kpo: p.cost,
            })
            .unwrap_or_else(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;

    fn adapter() -> JourneymanTypeAdapter {
        JourneymanTypeAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    #[test]
    fn le_journalier_est_la_ligne_marquee_is_journeyman() {
        let j = adapter().journeyman_type_for_roster("DEMO_GRANIT");
        assert_eq!(j.position_name, "Piétaille des Carrières");
        assert_eq!(j.price_kpo, 50);
    }

    #[test]
    fn un_roster_inconnu_retombe_sur_un_defaut_sans_paniquer() {
        let j = adapter().journeyman_type_for_roster("ROSTER_INEXISTANT");
        assert_eq!(j.price_kpo, 0);
    }
}
