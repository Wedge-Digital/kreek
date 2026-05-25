use std::sync::Arc;
use sqlx::PgPool;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::io::app_events::app_event_publisher::competitions_app_event_publisher;
use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
use crate::lib::services::event_bus::event_bus::EventBus;

#[derive(Clone)]
pub struct CompetitionsContext {
    pub competition_repository: Arc<dyn ICompetitionRepository>,
    pub event_bus:              EventBus,
}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    competitions_app_event_publisher(event_bus, app_event_bus);
}

impl CompetitionsContext {
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        Self {
            competition_repository: Arc::new(CompetitionRepository::new(pool.clone())),
            event_bus,
        }
    }
}