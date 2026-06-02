use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::app_events::app_event_publisher::competitions_app_event_publisher;
use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::lib::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct CompetitionsContext {
    pub competition_repository: Arc<dyn ICompetitionRepository>,
    pub season_repository: Arc<dyn ISeasonRepository>,
    pub event_bus: EventBus,
}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    competitions_app_event_publisher(event_bus, app_event_bus);
}

impl CompetitionsContext {
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        Self {
            competition_repository: Arc::new(CompetitionRepository::new(pool.clone())),
            season_repository: Arc::new(SeasonRepository::new(pool.clone())),
            event_bus,
        }
    }
}
