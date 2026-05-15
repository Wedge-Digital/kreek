use std::sync::Arc;
use sqlx::PgPool;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;

#[derive(Clone)]
pub struct SpacesContext {
    pub space_repository:      Arc<dyn ISpaceRepository>,
    pub user_cache_repository: Arc<dyn ISpaceUserCacheRepository>,
}

impl SpacesContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            space_repository:      Arc::new(SpaceRepository::new(pool.clone())),
            user_cache_repository: Arc::new(SpaceUserCacheRepository::new(pool.clone())),
        }
    }
}