use std::sync::Arc;
use sqlx::PgPool;
use crate::app::competitions::domain::cache_repository_port::ICompetitionsCacheRepository;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::io::app_event_listeners::space_created_listener::space_created_listener;
use crate::app::competitions::io::app_event_listeners::user_created_listener::user_created_listener;
use crate::app::competitions::io::app_event_listeners::user_subscribed_listener::user_subscribed_listener;
use crate::app::competitions::io::app_events::app_event_publisher::competitions_app_event_publisher;
use crate::app::competitions::io::repository::cache_repository::CompetitionsCacheRepository;
use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
use crate::lib::services::event_bus::event_bus::EventBus;

#[derive(Clone)]
pub struct CompetitionsContext {
    pub competitions_cache_repository: Arc<dyn ICompetitionsCacheRepository>,
    pub competition_repository:        Arc<dyn ICompetitionRepository>,
    pub event_bus:                     EventBus,
}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    competitions_app_event_publisher(event_bus, app_event_bus);
}

pub fn init_app_event_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo: Arc<dyn ICompetitionsCacheRepository> = Arc::new(CompetitionsCacheRepository::new(pool));
    user_created_listener(app_event_bus, Arc::clone(&repo));
    space_created_listener(app_event_bus, Arc::clone(&repo));
    user_subscribed_listener(app_event_bus, repo);
}

impl CompetitionsContext {
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        tracing::info!("CompetitionsContext::new");
        Self {
            competitions_cache_repository: Arc::new(CompetitionsCacheRepository::new(pool.clone())),
            competition_repository:        Arc::new(CompetitionRepository::new(pool.clone())),
            event_bus,
        }
    }
}