use std::sync::Arc;
use sqlx::PgPool;
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::ITeamRepository;

#[derive(Clone)]
pub struct TeamsContext {
    pub team_repository: Arc<dyn ITeamRepository>,
}

impl TeamsContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            team_repository: Arc::new(TeamRepository::new(pool.clone())),
        }
    }
}
