use crate::app::teams::io::app_events::app_event_publisher::teams_app_event_publisher;
use crate::app::teams::io::app_events::{
    match_report_cancelled_listener, match_report_confirmed_listener,
    match_report_published_listener, match_report_unpublished_listener, team_created_listener,
};
use crate::app::teams::io::listeners::team_value_listener::TeamValueDeps;
use crate::app::teams::io::listeners::{phase_basket_purge_listener, team_value_listener};
use crate::app::teams::io::repository::phase_basket_repository::PhaseBasketRepository;
use crate::app::teams::io::repository::team_repository::TeamRepository;
use crate::app::teams::ports::{
    IJourneymanTypePort, IPhaseBasketRepository, IRosterCatalogPort, ISquadPort, ITeamRepository,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct TeamsContext {
    pub team_repository: Arc<dyn ITeamRepository>,
    pub journeyman_type_port: Arc<dyn IJourneymanTypePort>,
    pub roster_catalog_port: Arc<dyn IRosterCatalogPort>,
    pub squad_port: Arc<dyn ISquadPort>,
    pub basket_repository: Arc<dyn IPhaseBasketRepository>,
}

pub fn init_listeners(
    app_event_bus: &EventBus,
    event_bus: &EventBus,
    pool: PgPool,
    squad_port: Arc<dyn ISquadPort>,
    roster_catalog_port: Arc<dyn IRosterCatalogPort>,
    journeyman_type_port: Arc<dyn IJourneymanTypePort>,
) {
    // Le publisher d'abord : il doit être abonné au bus interne avant que le
    // moindre événement domaine y passe.
    teams_app_event_publisher(event_bus, app_event_bus.clone(), pool.clone());

    let baskets: Arc<dyn IPhaseBasketRepository> =
        Arc::new(PhaseBasketRepository::new(pool.clone()));
    phase_basket_purge_listener::init(event_bus, baskets);
    let repo = Arc::new(TeamRepository::new(pool, event_bus.clone()));
    // Un seul jeu de dépendances pour les deux abonnements : la valeur d'équipe
    // se recalcule à l'identique, que le déclencheur vienne du bus interne ou de
    // l'annonce de `players`.
    let valeur = TeamValueDeps {
        repo: repo.clone(),
        squad_port,
        roster_catalog_port,
        journeyman_type_port,
    };
    team_value_listener::init(event_bus, valeur.clone());
    team_value_listener::init_from_app_events(app_event_bus, valeur);
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
        journeyman_type_port: Arc<dyn IJourneymanTypePort>,
        roster_catalog_port: Arc<dyn IRosterCatalogPort>,
        squad_port: Arc<dyn ISquadPort>,
    ) -> Self {
        Self {
            team_repository: Arc::new(TeamRepository::new(pool.clone(), event_bus)),
            journeyman_type_port,
            roster_catalog_port,
            squad_port,
            basket_repository: Arc::new(PhaseBasketRepository::new(pool.clone())),
        }
    }
}
