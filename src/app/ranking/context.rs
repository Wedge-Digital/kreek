use crate::app::ranking::io::app_events::{
    match_report_published_listener, match_report_round_reassigned_listener,
    match_report_unpublished_listener,
};
use crate::app::ranking::io::repository::ranking_repository::PgRankingRepository;
use crate::app::ranking::ports::{
    IRankingCompetitionPort, IRankingRepository, IRankingTeamLinksPort,
};
use crate::app::shared_kernel::bloodbowl::admin_access::IAdminAccessPort;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RankingContext {
    pub repository: Arc<dyn IRankingRepository>,
    pub competition_port: Arc<dyn IRankingCompetitionPort>,
    /// Qui peut attribuer ou retirer des points manuels (carte 450).
    pub admin_access: Arc<dyn IAdminAccessPort>,
    /// Les adresses de fiche et d'identité d'une équipe (carte 567).
    pub team_links_port: Arc<dyn IRankingTeamLinksPort>,
}

pub fn init_listeners(
    app_event_bus: &EventBus,
    pool: PgPool,
    competition_port: Arc<dyn IRankingCompetitionPort>,
) {
    let repo: Arc<dyn IRankingRepository> = Arc::new(PgRankingRepository::new(pool));
    match_report_unpublished_listener::init(app_event_bus, repo.clone());
    match_report_round_reassigned_listener::init(app_event_bus, repo.clone());
    match_report_published_listener::init(app_event_bus, repo, competition_port);
}

impl RankingContext {
    pub fn new(
        pool: &PgPool,
        competition_port: Arc<dyn IRankingCompetitionPort>,
        admin_access: Arc<dyn IAdminAccessPort>,
        team_links_port: Arc<dyn IRankingTeamLinksPort>,
    ) -> Self {
        Self {
            repository: Arc::new(PgRankingRepository::new(pool.clone())),
            competition_port,
            admin_access,
            team_links_port,
        }
    }
}
