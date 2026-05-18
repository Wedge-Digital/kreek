use std::sync::{Arc, Mutex};
use sqlx::PgPool;
use crate::app::competitions::domain::cache_repository_port::ICompetitionsCacheRepository;
use crate::app::competitions::io::app_event_listeners::space_created_listener::space_created_listener;
use crate::app::competitions::io::app_event_listeners::user_created_listener::user_created_listener;
use crate::app::competitions::io::app_event_listeners::user_subscribed_listener::user_subscribed_listener;
use crate::app::competitions::io::repository::cache_repository::CompetitionsCacheRepository;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;
use crate::lib::services::event_bus::event_bus::{EventBus, IEventPublisher};

#[derive(Clone)]
pub struct CompetitionsContext {
    pub competitions_cache_repository:  Arc<dyn ICompetitionsCacheRepository>,
    pub event_bus:                      Arc<Mutex<dyn IEventPublisher>>,
}

pub fn init_app_event_listeners(app_event_bus: Arc<Mutex<EventBus>>, pool: PgPool) {
    let repo: Arc<dyn ICompetitionsCacheRepository> = Arc::new(CompetitionsCacheRepository::new(pool));
    user_created_listener(Arc::clone(&app_event_bus), Arc::clone(&repo));
    space_created_listener(Arc::clone(&app_event_bus), Arc::clone(&repo));
    user_subscribed_listener(Arc::clone(&app_event_bus), repo);
}

impl CompetitionsContext {
        pub fn new(pool: &PgPool, event_bus: Arc<Mutex<dyn IEventPublisher>>) -> Self {
            tracing::info!("CompetitionsContext::new");
            Self {
                competitions_cache_repository: Arc::new(CompetitionsCacheRepository::new(pool.clone())),
                event_bus
            }
    }
}