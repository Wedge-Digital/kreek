use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::io::repository::match_report_repository::MatchReportRepository;
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct MatchReportContext {
    pub match_report_repo: Arc<dyn IMatchReportRepository>,
    pub competition_data: Arc<dyn ICompetitionDataPort>,
    pub team_data: Arc<dyn ITeamDataPort>,
}

impl MatchReportContext {
    pub fn new(
        pool: &PgPool,
        competition_data: Arc<dyn ICompetitionDataPort>,
        team_data: Arc<dyn ITeamDataPort>,
    ) -> Self {
        Self {
            match_report_repo: Arc::new(MatchReportRepository::new(pool.clone())),
            competition_data,
            team_data,
        }
    }
}
