use std::sync::Arc;
use sqlx::PgPool;
use crate::app::team_creation::io::team_creation_repository::TeamDraftRepository;
use crate::app::team_creation::ports::ITeamDraftRepository;

#[derive(Clone)]
pub struct TeamCreationContext {
    pub team_repository: Arc<dyn ITeamDraftRepository>,
}

impl TeamCreationContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            team_repository: Arc::new(TeamDraftRepository::new(pool.clone())),
        }
    }
}