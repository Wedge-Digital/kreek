use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::ISpaceUserCacheRepository;
use crate::app::spaces::io::app_events::app_event_publisher::spaces_app_event_publisher;
use crate::app::spaces::io::app_events::user_created_listener::user_created_listener;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct SpacesContext {
    pub space_repository: Arc<dyn ISpaceRepository>,
    pub user_cache_repository: Arc<dyn ISpaceUserCacheRepository>,
    pub event_bus: EventBus,
}

pub fn init_app_event_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(SpaceUserCacheRepository::new(pool));
    user_created_listener(app_event_bus, repo);
}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    spaces_app_event_publisher(event_bus, app_event_bus);
}

impl SpacesContext {
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        Self {
            space_repository: Arc::new(SpaceRepository::new(pool.clone())),
            user_cache_repository: Arc::new(SpaceUserCacheRepository::new(pool.clone())),
            event_bus,
        }
    }
}
