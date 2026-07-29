use crate::app::auth::context::AuthContext;
use crate::app::competitions::context::CompetitionsContext;
use crate::app::match_report::context::MatchReportContext;
use crate::app::news::context::NewsContext;
use crate::app::players::context::PlayersContext;
use crate::app::ranking::context::RankingContext;
use crate::app::references::context::ReferencesContext;
use crate::app::spaces::context::SpacesContext;
use crate::app::team_creation::context::TeamCreationContext;
use crate::app::teams::context::TeamsContext;
use crate::common::services::event_bus::event_bus::EventBus;
use axum::extract::FromRef;

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthContext,
    pub spaces: SpacesContext,
    pub competitions: CompetitionsContext,
    pub match_report: MatchReportContext,
    pub news: NewsContext,
    pub references: ReferencesContext,
    pub team_creation: TeamCreationContext,
    pub teams: TeamsContext,
    pub players: PlayersContext,
    pub ranking: RankingContext,
    pub bypass_auth: bool,
    pub event_bus: EventBus,
    pub app_event_bus: EventBus,
}

// Les handlers d'auth et de spaces ne prennent que leur propre contexte : sans
// ces projections, un handler de login dépendrait au niveau du type des dix
// BCs de l'application, et son test devrait tous les construire.
//
// Ces `impl` vivent ici et non dans les contextes des BCs : c'est `AppState`
// qui connaît ses parties, l'inverse rendrait `auth` et `spaces` inextricables
// du reste — précisément ce que la série d'extraction supprime.
impl FromRef<AppState> for AuthContext {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for SpacesContext {
    fn from_ref(state: &AppState) -> Self {
        state.spaces.clone()
    }
}
