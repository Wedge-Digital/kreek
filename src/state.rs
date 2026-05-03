use std::sync::Arc;
use crate::app::auth::ports::IUserRepository;
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::lib::services::email::IEmailService;

#[derive(Clone)]
pub struct AppState {
    pub user_repository:        Arc<dyn IUserRepository>,
    pub reset_token_repository: Arc<dyn IResetTokenRepository>,
    pub email_service:          Arc<dyn IEmailService>,
    pub host_domain:            String,   
}
