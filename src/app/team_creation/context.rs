use crate::app::team_creation::io::app_events::app_event_publisher::team_creation_app_event_publisher;
use crate::app::team_creation::io::team_creation_repository::{
    TeamDraftRepository, TeamRosterRepository,
};
use crate::app::team_creation::ports::{
    ICompetitionCreationRulesPort, ICompetitionDisplayPort, IReferenceDataPort,
    ITeamDraftRepository, ITeamRosterRepository,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct TeamCreationContext {
    pub team_repository: Arc<dyn ITeamDraftRepository>,
    pub roster_repository: Arc<dyn ITeamRosterRepository>,
    pub reference_data: Arc<dyn IReferenceDataPort>,
    pub competition_rules: Arc<dyn ICompetitionCreationRulesPort>,
    pub competition_display: Arc<dyn ICompetitionDisplayPort>,
    pub event_bus: EventBus,
}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    team_creation_app_event_publisher(event_bus, app_event_bus);
}

impl TeamCreationContext {
    pub fn new(
        pool: &PgPool,
        event_bus: EventBus,
        reference_data: Arc<dyn IReferenceDataPort>,
        competition_rules: Arc<dyn ICompetitionCreationRulesPort>,
        competition_display: Arc<dyn ICompetitionDisplayPort>,
    ) -> Self {
        Self {
            team_repository: Arc::new(TeamDraftRepository::new(pool.clone())),
            roster_repository: Arc::new(TeamRosterRepository::new(pool.clone())),
            reference_data,
            competition_rules,
            competition_display,
            event_bus,
        }
    }
}
