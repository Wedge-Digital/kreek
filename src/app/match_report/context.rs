use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::io::app_events::{pairing_created_listener, pairing_deleted_listener};
use crate::app::match_report::io::repository::match_report_repository::MatchReportRepository;
use crate::app::match_report::ports::{ICompetitionDataPort, IPlayerDataPort, ITeamDataPort};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct MatchReportContext {
    pub match_report_repo: Arc<dyn IMatchReportRepository>,
    pub competition_data: Arc<dyn ICompetitionDataPort>,
    pub team_data: Arc<dyn ITeamDataPort>,
    pub player_data: Arc<dyn IPlayerDataPort>,
}

pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(MatchReportRepository::new(pool));
    pairing_created_listener::init(app_event_bus, repo.clone());
    pairing_deleted_listener::init(app_event_bus, repo);
}

impl MatchReportContext {
    pub fn new(
        pool: &PgPool,
        competition_data: Arc<dyn ICompetitionDataPort>,
        team_data: Arc<dyn ITeamDataPort>,
        player_data: Arc<dyn IPlayerDataPort>,
    ) -> Self {
        Self {
            match_report_repo: Arc::new(MatchReportRepository::new(pool.clone())),
            competition_data,
            team_data,
            player_data,
        }
    }
}
