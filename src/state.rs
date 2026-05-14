use std::sync::Arc;
use crate::app::auth::ports::IUserRepository;
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::app::spaces::domain::ports::ISpaceRepository;
use crate::lib::services::email::IEmailService;
use crate::lib::services::event_bus::event_bus::EventBus;

#[derive(Clone)]
pub struct AppState {
    pub user_repository:        Arc<dyn IUserRepository>,
    pub reset_token_repository: Arc<dyn IResetTokenRepository>,
    pub space_repository:       Arc<dyn ISpaceRepository>,
    pub email_service:          Arc<dyn IEmailService>,
    pub host_domain:            String,
    pub bypass_auth:            bool,
    pub domain_event_bus:       Arc<EventBus>,
}