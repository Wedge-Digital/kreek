use crate::app::players::io::app_events::app_event_publisher::players_app_event_publisher;
use crate::app::players::io::app_events::{
    player_match_impact_listener, player_recruited_listener, team_created_listener,
};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
use crate::app::players::ports::{
    IPlayerCompetitionPort, IPlayerProjectionRepository, IPlayerRepository, IPlayerRosterPort,
    IPlayerSpaceMemberPort, ISkillCatalogPort,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlayersContext {
    pub repository: Arc<dyn IPlayerRepository>,
    pub projection_repository: Arc<dyn IPlayerProjectionRepository>,
    pub skill_catalog: Arc<dyn ISkillCatalogPort>,
    pub roster_port: Arc<dyn IPlayerRosterPort>,
    pub competition_port: Arc<dyn IPlayerCompetitionPort>,
    pub space_member_port: Arc<dyn IPlayerSpaceMemberPort>,
    pub event_bus: EventBus,
}

impl PlayersContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: &PgPool,
        skill_catalog: Arc<dyn ISkillCatalogPort>,
        roster_port: Arc<dyn IPlayerRosterPort>,
        competition_port: Arc<dyn IPlayerCompetitionPort>,
        space_member_port: Arc<dyn IPlayerSpaceMemberPort>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            repository: Arc::new(PgPlayerRepository::new(pool.clone())),
            projection_repository: Arc::new(PgPlayerProjectionRepository::new(pool.clone())),
            skill_catalog,
            roster_port,
            competition_port,
            space_member_port,
            event_bus,
        }
    }
}

pub fn init_listeners(
    event_bus: &EventBus,
    app_event_bus: &EventBus,
    pool: PgPool,
    skill_catalog: Arc<dyn ISkillCatalogPort>,
) {
    players_app_event_publisher(event_bus, app_event_bus.clone());
    let player_repo: Arc<dyn IPlayerRepository> = Arc::new(PgPlayerRepository::new(pool.clone()));
    team_created_listener::init(
        app_event_bus,
        event_bus.clone(),
        pool.clone(),
        skill_catalog.clone(),
    );
    player_recruited_listener::init(app_event_bus, pool, skill_catalog.clone());
    player_match_impact_listener::init(app_event_bus, player_repo, skill_catalog);
}
