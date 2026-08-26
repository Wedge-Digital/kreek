use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::group_repository_port::IGroupRepository;
use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::app_events::app_event_publisher::competitions_app_event_publisher;
use crate::app::competitions::io::app_events::competition_ready_listener;
use crate::app::competitions::io::app_events::match_report_cancelled_listener;
use crate::app::competitions::io::app_events::match_report_confirmed_listener;
use crate::app::competitions::io::app_events::match_report_published_listener;
use crate::app::competitions::io::app_events::match_report_unpublished_listener;
use crate::app::competitions::io::app_events::user_unsubscribed_listener;
use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
use crate::app::competitions::io::repository::group_repository::GroupRepository;
use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::app::competitions::ports::{
    ICompetitionReferencePort, ICompetitionSpaceMemberPort, IMatchReportStatusPort, ITeamInfoPort,
    ITiebreakCatalogPort,
};
use crate::common::services::email::IEmailService;
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
    pub reference_port: Arc<dyn ICompetitionReferencePort>,
    pub space_member_port: Arc<dyn ICompetitionSpaceMemberPort>,
    pub tiebreak_catalog_port: Arc<dyn ITiebreakCatalogPort>,
    pub match_report_status_port: Arc<dyn IMatchReportStatusPort>,
    pub email_service: Arc<dyn IEmailService>,
    /// L'URL publique, schéma compris, pour les liens des e-mails.
    ///
    /// Normalisée une fois par `AppConfig::app_url()` et injectée telle quelle,
    /// ici comme dans `auth`. Personne ne recolle plus de schéma.
    pub app_url: String,
    pub event_bus: EventBus,
}

pub fn init_listeners(
    event_bus: &EventBus,
    app_event_bus: EventBus,
    pool: PgPool,
    match_day_repository: Arc<dyn IMatchDayRepository>,
    team_info_port: Arc<dyn ITeamInfoPort>,
) {
    // Le listener de confirmation reçoit le dépôt de journées et le port
    // équipes depuis la carte 427 : il fabrique désormais l'appariement d'un
    // rapport manuel, comme le fait celui de publication.
    match_report_confirmed_listener::init(
        &app_event_bus,
        event_bus.clone(),
        pool.clone(),
        match_day_repository.clone(),
        team_info_port.clone(),
    );
    match_report_cancelled_listener::init(
        &app_event_bus,
        pool.clone(),
        match_day_repository.clone(),
    );
    user_unsubscribed_listener::init(&app_event_bus, pool.clone());
    match_report_unpublished_listener::init(&app_event_bus, pool.clone());
    match_report_published_listener::init(
        &app_event_bus,
        event_bus.clone(),
        pool,
        match_day_repository,
        team_info_port,
    );
    competitions_app_event_publisher(event_bus, app_event_bus);
}

/// Le listener d'ouverture, monté à part : il a besoin du service d'e-mail et de
/// l'URL publique, que `init_listeners` n'a pas.
///
/// `init(event_bus: …)` **sans** préfixe `app_` — c'est le bus interne du BC, et
/// l'axe 5 de `check-arch` lit cette signature pour distinguer un listener
/// intra-BC d'un listener cross-BC.
pub fn init_registration_open_listener(
    event_bus: &EventBus,
    deps: Arc<competition_ready_listener::RegistrationOpenDeps>,
) {
    competition_ready_listener::init(event_bus, deps);
}

impl CompetitionsContext {
    pub fn new(
        pool: &PgPool,
        event_bus: EventBus,
        team_info_port: Arc<dyn ITeamInfoPort>,
        reference_port: Arc<dyn ICompetitionReferencePort>,
        space_member_port: Arc<dyn ICompetitionSpaceMemberPort>,
        tiebreak_catalog_port: Arc<dyn ITiebreakCatalogPort>,
        match_report_status_port: Arc<dyn IMatchReportStatusPort>,
        email_service: Arc<dyn IEmailService>,
        app_url: String,
    ) -> Self {
        Self {
            competition_repository: Arc::new(CompetitionRepository::new(pool.clone())),
            season_repository: Arc::new(SeasonRepository::new(pool.clone())),
            group_repository: Arc::new(GroupRepository::new(pool.clone())),
            match_day_repository: Arc::new(MatchDayRepository::new(pool.clone())),
            team_info_port,
            reference_port,
            space_member_port,
            tiebreak_catalog_port,
            match_report_status_port,
            email_service,
            app_url,
            event_bus,
        }
    }
}
