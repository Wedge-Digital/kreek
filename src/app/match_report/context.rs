use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::io::app_events::{
    app_event_publisher, pairing_created_listener, pairing_deleted_listener,
};
use crate::app::match_report::io::repository::match_report_repository::MatchReportRepository;
use crate::app::match_report::ports::{
    ICoachDataPort, ICompetitionDataPort, IKeywordCatalogPort, IPlayerDataPort, ISpaceAdminPort,
    ISppCalculatorPort, ITeamDataPort,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct MatchReportContext {
    pub match_report_repo: Arc<dyn IMatchReportRepository>,
    pub competition_data: Arc<dyn ICompetitionDataPort>,
    pub team_data: Arc<dyn ITeamDataPort>,
    pub player_data: Arc<dyn IPlayerDataPort>,
    pub coach_data: Arc<dyn ICoachDataPort>,
    pub space_admin: Arc<dyn ISpaceAdminPort>,
    pub spp_calculator: Arc<dyn ISppCalculatorPort>,
    pub keyword_catalog: Arc<dyn IKeywordCatalogPort>,
    pub event_bus: EventBus,
}

pub fn init_listeners(
    event_bus: &EventBus,
    app_event_bus: &EventBus,
    pool: PgPool,
    competition_data: Arc<dyn ICompetitionDataPort>,
    team_data: Arc<dyn ITeamDataPort>,
) {
    let repo = Arc::new(MatchReportRepository::new(pool));
    pairing_created_listener::init(app_event_bus, repo.clone());
    pairing_deleted_listener::init(app_event_bus, event_bus, repo.clone());
    app_event_publisher::match_report_app_event_publisher(
        event_bus,
        app_event_bus.clone(),
        repo,
        competition_data,
        team_data,
    );
}

impl MatchReportContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: &PgPool,
        competition_data: Arc<dyn ICompetitionDataPort>,
        team_data: Arc<dyn ITeamDataPort>,
        player_data: Arc<dyn IPlayerDataPort>,
        coach_data: Arc<dyn ICoachDataPort>,
        space_admin: Arc<dyn ISpaceAdminPort>,
        spp_calculator: Arc<dyn ISppCalculatorPort>,
        keyword_catalog: Arc<dyn IKeywordCatalogPort>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            match_report_repo: Arc::new(MatchReportRepository::new(pool.clone())),
            competition_data,
            team_data,
            player_data,
            coach_data,
            space_admin,
            spp_calculator,
            keyword_catalog,
            event_bus,
        }
    }
}
