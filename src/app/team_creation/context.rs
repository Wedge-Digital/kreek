use std::sync::Arc;
use sqlx::PgPool;
use crate::app::team_creation::io::team_creation_repository::{TeamDraftRepository, TeamRosterRepository};
use crate::app::team_creation::ports::{ITeamDraftRepository, ITeamRosterRepository};

#[derive(Clone)]
pub struct TeamCreationContext {
    pub team_repository:   Arc<dyn ITeamDraftRepository>,
    pub roster_repository: Arc<dyn ITeamRosterRepository>,
}

impl TeamCreationContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            team_repository:   Arc::new(TeamDraftRepository::new(pool.clone())),
            roster_repository: Arc::new(TeamRosterRepository::new(pool.clone())),
        }
    }
}