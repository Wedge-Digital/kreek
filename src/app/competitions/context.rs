use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::io::app_events::app_event_publisher::competitions_app_event_publisher;
use crate::app::competitions::io::app_events::match_report_confirmed_listener;
use crate::app::competitions::io::app_events::match_report_published_listener;
use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
use crate::app::competitions::io::repository::group_repository::GroupRepository;
use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct CompetitionsContext {
    pub competition_repository: Arc<dyn ICompetitionRepository>,
    pub season_repository: Arc<dyn ISeasonRepository>,
    pub group_repository: Arc<dyn IGroupRepository>,
    pub match_day_repository: Arc<dyn IMatchDayRepository>,
    pub team_info_port: Arc<dyn ITeamInfoPort>,
    pub event_bus: EventBus,
}

pub fn init_listeners(
    event_bus: &EventBus,
    app_event_bus: EventBus,
    pool: PgPool,
    match_day_repository: Arc<dyn IMatchDayRepository>,
    team_info_port: Arc<dyn ITeamInfoPort>,
) {
    match_report_confirmed_listener::init(&app_event_bus, pool.clone());
    match_report_published_listener::init(&app_event_bus, event_bus.clone(), pool, match_day_repository, team_info_port);
    competitions_app_event_publisher(event_bus, app_event_bus);
}

impl CompetitionsContext {
    pub fn new(pool: &PgPool, event_bus: EventBus, team_info_port: Arc<dyn ITeamInfoPort>) -> Self {
        Self {
            competition_repository: Arc::new(CompetitionRepository::new(pool.clone())),
            season_repository: Arc::new(SeasonRepository::new(pool.clone())),
            group_repository: Arc::new(GroupRepository::new(pool.clone())),
            match_day_repository: Arc::new(MatchDayRepository::new(pool.clone())),
            team_info_port,
            event_bus,
        }
    }
}
