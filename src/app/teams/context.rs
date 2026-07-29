use crate::app::teams::io::app_events::{
    match_report_cancelled_listener, match_report_confirmed_listener,
    match_report_published_listener, match_report_unpublished_listener, team_created_listener,
};
use crate::app::teams::io::listeners::team_value_listener;
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::{
    IJourneymanTypePort, IPlayerCountPort, IPlayerValuePort, IRosterInfoPort, ITeamRepository,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct TeamsContext {
    pub team_repository: Arc<dyn ITeamRepository>,
    pub player_count_port: Arc<dyn IPlayerCountPort>,
    pub journeyman_type_port: Arc<dyn IJourneymanTypePort>,
    pub roster_info_port: Arc<dyn IRosterInfoPort>,
    pub player_value_port: Arc<dyn IPlayerValuePort>,
}

pub fn init_listeners(
    app_event_bus: &EventBus,
    event_bus: &EventBus,
    pool: PgPool,
    player_value_port: Arc<dyn IPlayerValuePort>,
    roster_info_port: Arc<dyn IRosterInfoPort>,
    journeyman_type_port: Arc<dyn IJourneymanTypePort>,
) {
    let repo = Arc::new(TeamRepository::new(pool, event_bus.clone()));
    team_value_listener::init(
        event_bus,
        repo.clone(),
        player_value_port,
        roster_info_port,
        journeyman_type_port,
    );
    team_created_listener::init(app_event_bus, repo.clone());
    match_report_confirmed_listener::init(app_event_bus, repo.clone());
    match_report_cancelled_listener::init(app_event_bus, repo.clone());
    match_report_published_listener::init(app_event_bus, repo.clone());
    match_report_unpublished_listener::init(app_event_bus, repo);
}

impl TeamsContext {
    pub fn new(
        pool: &PgPool,
        event_bus: EventBus,
        player_count_port: Arc<dyn IPlayerCountPort>,
        journeyman_type_port: Arc<dyn IJourneymanTypePort>,
        roster_info_port: Arc<dyn IRosterInfoPort>,
        player_value_port: Arc<dyn IPlayerValuePort>,
    ) -> Self {
        Self {
            team_repository: Arc::new(TeamRepository::new(pool.clone(), event_bus)),
            player_count_port,
            journeyman_type_port,
            roster_info_port,
            player_value_port,
        }
    }
}
