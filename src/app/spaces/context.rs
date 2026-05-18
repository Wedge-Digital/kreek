use std::sync::{Arc, Mutex};
use sqlx::PgPool;
use crate::app::spaces::io::app_event_publisher::spaces_app_event_publisher;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::io::app_event_listeners::user_created_listener::user_created_listener;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;
use crate::lib::services::event_bus::event_bus::{EventBus, IEventPublisher};

#[derive(Clone)]
pub struct SpacesContext {
    pub space_repository:       Arc<dyn ISpaceRepository>,
    pub user_cache_repository:  Arc<dyn ISpaceUserCacheRepository>,
    pub event_bus:       Arc<Mutex<dyn IEventPublisher>>,
}

pub fn init_app_event_listeners(app_event_bus: Arc<Mutex<EventBus>>, pool: PgPool) {
    let repo = Arc::new(SpaceUserCacheRepository::new(pool));
    user_created_listener(app_event_bus.clone(), repo);
}

pub fn init_app_event_publisher(app_event_bus: Arc<Mutex<EventBus>>, event_bus: Arc<Mutex<EventBus>>) {
    spaces_app_event_publisher(app_event_bus, event_bus);
}

impl SpacesContext {
pub fn new(pool: &PgPool, event_bus:  Arc<Mutex<dyn IEventPublisher>>,) -> Self {
        Self {
            space_repository:      Arc::new(SpaceRepository::new(pool.clone())),
            user_cache_repository: Arc::new(SpaceUserCacheRepository::new(pool.clone())),
            event_bus,
        }
    }
}