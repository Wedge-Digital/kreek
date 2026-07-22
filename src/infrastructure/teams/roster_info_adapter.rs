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
}

impl IRosterInfoPort for RosterInfoAdapter {
    fn find_roster_info(&self, roster_id: &str) -> Option<RosterInfoDto> {
        let team = self.refs.find_team_by_uid(roster_id)?;
        Some(RosterInfoDto {
            logo:        team.logo.clone(),
            reroll_cost: team.reroll_cost,
        })
    }
}
