use std::sync::Arc;
use crate::app::auth::ports::IUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_repository: Arc<dyn IUserRepository>,
}
