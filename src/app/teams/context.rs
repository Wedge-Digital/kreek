use crate::app::teams::io::app_events::{
    match_report_confirmed_listener, match_report_published_listener, player_improvement_listener,
    team_created_listener,
};
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::{IJourneymanTypePort, IPlayerCountPort, ITeamRepository};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct TeamsContext {
    pub team_repository: Arc<dyn ITeamRepository>,
    pub player_count_port: Arc<dyn IPlayerCountPort>,
    pub journeyman_type_port: Arc<dyn IJourneymanTypePort>,
}

pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(TeamRepository::new(pool));
    team_created_listener::init(app_event_bus, repo.clone());
    match_report_confirmed_listener::init(app_event_bus, repo.clone());
    match_report_published_listener::init(app_event_bus, repo.clone());
    player_improvement_listener::init(app_event_bus, repo);
}

impl TeamsContext {
    pub fn new(
        pool: &PgPool,
        player_count_port: Arc<dyn IPlayerCountPort>,
        journeyman_type_port: Arc<dyn IJourneymanTypePort>,
    ) -> Self {
        Self {
            team_repository: Arc::new(TeamRepository::new(pool.clone())),
            player_count_port,
            journeyman_type_port,
        }
    }
}
