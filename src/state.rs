use crate::app::auth::context::AuthContext;
use crate::app::competitions::context::CompetitionsContext;
use crate::app::match_report::context::MatchReportContext;
use crate::app::news::context::NewsContext;
use crate::app::references::context::ReferencesContext;
use crate::app::spaces::context::SpacesContext;
use crate::app::team_creation::context::TeamCreationContext;
use crate::app::players::context::PlayersContext;
use crate::app::teams::context::TeamsContext;
use crate::common::services::email::IEmailService;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthContext,
    pub spaces: SpacesContext,
    pub competitions: CompetitionsContext,
    pub match_report: MatchReportContext,
    pub news: NewsContext,
    pub references: ReferencesContext,
    pub team_creation: TeamCreationContext,
    pub teams:   TeamsContext,
    pub players: PlayersContext,
    pub email_service: Arc<dyn IEmailService>,
    pub host_domain: String,
    pub bypass_auth: bool,
    pub event_bus: EventBus,
    pub app_event_bus: EventBus,
}
