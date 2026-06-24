use crate::app::auth::routes::Routes as AuthRoutes;
use crate::app::competitions::routes::Routes as CompetitionsRoutes;
use crate::app::match_report::routes::Routes as MatchReportRoutes;
use crate::app::news::routes::Routes as NewsRoutes;
use crate::app::players::routes::Routes as PlayersRoutes;
use crate::app::references::routes::Routes as ReferencesRoutes;
use crate::app::spaces::routes::Routes as SpacesRoutes;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::teams::routes::Routes as TeamsRoutes;
use crate::web::routes::Routes as WebRoutes;

/// Agrège toutes les routes de l'application.
///
/// `AppRoutes` est `Copy + Default` — coût zéro à construire.
/// Utiliser `AppRoutes::default()` dans les handlers et `routes: AppRoutes`
/// comme unique champ de routes dans les structs de template Askama.
#[derive(Debug, Clone, Copy, Default)]
pub struct AppRoutes {
    pub auth:          AuthRoutes,
    pub web:           WebRoutes,
    pub competitions:  CompetitionsRoutes,
    pub match_report:  MatchReportRoutes,
    pub news:          NewsRoutes,
    pub spaces:        SpacesRoutes,
    pub team_creation: TeamCreationRoutes,
    pub teams:         TeamsRoutes,
    pub players:       PlayersRoutes,
    pub references:    ReferencesRoutes,
}