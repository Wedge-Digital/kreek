use std::sync::Arc;
use sqlx::PgPool;
use crate::app::teams::io::app_events::team_created_listener;
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::ITeamRepository;
use crate::lib::services::event_bus::event_bus::EventBus;

#[derive(Clone)]
pub struct TeamsContext {
    pub team_repository: Arc<dyn ITeamRepository>,
}

pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(TeamRepository::new(pool));
    team_created_listener::init(app_event_bus, repo);
}

impl TeamsContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            team_repository: Arc::new(TeamRepository::new(pool.clone())),
        }
    }
}
