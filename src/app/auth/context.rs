use crate::app::auth::io::app_events::app_event_publisher::auth_app_event_publisher;
use crate::app::auth::io::repository::reset_token_repository::{
    IResetTokenRepository, ResetTokenRepository,
};
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::ports::IUserRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthContext {
    pub user_repository: Arc<dyn IUserRepository>,
    pub reset_token_repository: Arc<dyn IResetTokenRepository>,
    pub event_bus: EventBus,
}

pub fn init_app_event_listeners(_app_event_bus: &EventBus, _event_bus: &EventBus, _pool: PgPool) {}

pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    auth_app_event_publisher(event_bus, app_event_bus);
}

impl AuthContext {
    pub fn new(pool: &PgPool, event_bus: EventBus) -> Self {
        Self {
            user_repository: Arc::new(UserRepository::new(pool.clone())),
            reset_token_repository: Arc::new(ResetTokenRepository::new(pool.clone())),
            event_bus,
        }
    }
}
