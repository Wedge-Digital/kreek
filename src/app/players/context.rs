use crate::app::players::io::app_events::app_event_publisher::players_app_event_publisher;
use crate::app::players::io::app_events::{player_match_impact_listener, team_created_listener};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
use crate::app::players::ports::{IPlayerProjectionRepository, IPlayerRepository, ISkillCatalogPort};
use crate::app::references::domain::port::IReferenceRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlayersContext {
    pub repository:            Arc<dyn IPlayerRepository>,
    pub projection_repository: Arc<dyn IPlayerProjectionRepository>,
    pub skill_catalog:         Arc<dyn ISkillCatalogPort>,
    pub event_bus:             EventBus,
}

impl PlayersContext {
    pub fn new(pool: &PgPool, skill_catalog: Arc<dyn ISkillCatalogPort>, event_bus: EventBus) -> Self {
        Self {
            repository:            Arc::new(PgPlayerRepository::new(pool.clone())),
            projection_repository: Arc::new(PgPlayerProjectionRepository::new(pool.clone())),
            skill_catalog,
            event_bus,
        }
    }
}

pub fn init_listeners(
    event_bus:     &EventBus,
    app_event_bus: &EventBus,
    pool:          PgPool,
    refs:          Arc<dyn IReferenceRepository>,
) {
    let player_repo: Arc<dyn IPlayerRepository> = Arc::new(PgPlayerRepository::new(pool.clone()));
    team_created_listener::init(app_event_bus, pool, refs.clone());
    player_match_impact_listener::init(app_event_bus, player_repo, refs);
    players_app_event_publisher(event_bus, app_event_bus.clone());
}
