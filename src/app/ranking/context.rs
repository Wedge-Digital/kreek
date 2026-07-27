use crate::app::ranking::io::app_events::{
    match_report_published_listener, match_report_unpublished_listener,
};
use crate::app::ranking::io::repository::ranking_repository::PgRankingRepository;
use crate::app::ranking::ports::{IRankingCompetitionPort, IRankingRepository};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RankingContext {
    pub repository: Arc<dyn IRankingRepository>,
    pub competition_port: Arc<dyn IRankingCompetitionPort>,
}

pub fn init_listeners(
    app_event_bus: &EventBus,
    pool: PgPool,
    competition_port: Arc<dyn IRankingCompetitionPort>,
) {
    let repo: Arc<dyn IRankingRepository> = Arc::new(PgRankingRepository::new(pool));
    match_report_unpublished_listener::init(app_event_bus, repo.clone());
    match_report_published_listener::init(app_event_bus, repo, competition_port);
}

impl RankingContext {
    pub fn new(pool: &PgPool, competition_port: Arc<dyn IRankingCompetitionPort>) -> Self {
        Self {
            repository: Arc::new(PgRankingRepository::new(pool.clone())),
            competition_port,
        }
    }
}
