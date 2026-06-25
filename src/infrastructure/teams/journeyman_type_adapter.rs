use crate::app::references::domain::port::IReferenceRepository;
use crate::app::teams::ports::IJourneymanTypePort;
use std::sync::Arc;

pub struct JourneymanTypeAdapter {
    refs: Arc<dyn IReferenceRepository>,
}

impl JourneymanTypeAdapter {
    pub fn new(refs: Arc<dyn IReferenceRepository>) -> Self {
        Self { refs }
    }
}

impl IJourneymanTypePort for JourneymanTypeAdapter {
    fn journeyman_type_for_roster(&self, roster_id: &str) -> String {
        let Some(team) = self.refs.find_team_by_uid(roster_id) else {
            return "Lineman".to_string();
        };

        team.available_players
            .iter()
            .max_by_key(|p| p.max_quantity)
            .map(|p| p.position_name.clone())
            .unwrap_or_else(|| "Lineman".to_string())
    }
}
