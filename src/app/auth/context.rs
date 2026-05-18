use std::sync::{Arc, Mutex};
use sqlx::PgPool;
use crate::app::auth::io::app_event_publisher::auth_app_event_publisher;
use crate::app::auth::io::repository::reset_token_repository::{IResetTokenRepository, ResetTokenRepository};
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::ports::IUserRepository;
use crate::lib::services::event_bus::event_bus::{EventBus, IEventPublisher};

#[derive(Clone)]
pub struct AuthContext {
    pub user_repository:        Arc<dyn IUserRepository>,
    pub reset_token_repository: Arc<dyn IResetTokenRepository>,
    pub event_bus:              Arc<Mutex<dyn IEventPublisher>>,
}

pub fn init_app_event_listeners(app_event_bus: Arc<Mutex<EventBus>>, event_bus: Arc<Mutex<EventBus>>, pool: PgPool) {
}

pub fn init_app_event_publisher(app_event_bus: Arc<Mutex<EventBus>>, event_bus: Arc<Mutex<EventBus>>) {
    auth_app_event_publisher(app_event_bus, event_bus);
}

impl AuthContext {
    pub fn new(pool: &PgPool, event_bus: Arc<Mutex<dyn IEventPublisher>>) -> Self {
        Self {
            user_repository:        Arc::new(UserRepository::new(pool.clone())),
            reset_token_repository: Arc::new(ResetTokenRepository::new(pool.clone())),
            event_bus,
        }
    }
}