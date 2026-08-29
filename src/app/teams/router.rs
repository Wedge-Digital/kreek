use crate::app::teams::io::web::dismiss_team::dismiss_team;
use crate::app::teams::io::web::dismissals::dismissals_page;
use crate::app::teams::io::web::recruitment::recruitment_page;
use crate::app::teams::io::web::team_detail::{team_detail, team_page_treasury};
use crate::app::teams::io::web::validate_phase_actions::{
    post_validate_dismissals_phase, post_validate_improvement_phase,
    post_validate_recruitment_phase,
};
use crate::app::teams::io::web::widgets::competition_teams_widget::competition_teams_widget;
use crate::app::teams::io::web::widgets::dismissals_cart_widget::{dismissals_cart, unmark_staff};
use crate::app::teams::io::web::widgets::dismissals_roster_widget::{
    dismissals_roster, mark_player, mark_staff, unmark_player,
};
use crate::app::teams::io::web::widgets::enrolled_teams_widget::enrolled_teams_widget;
use crate::app::teams::io::web::widgets::enrollment_actions::{
    approve_all_enrollments, approve_enrollment, dismiss_enrollment, reject_enrollment,
};
use crate::app::teams::io::web::widgets::my_teams_widget::my_teams_widget;
use crate::app::teams::io::web::widgets::pending_enrollment_widget::pending_enrollment_widget;
use crate::app::teams::io::web::widgets::recruitment_cart_widget::{
    recruitment_cart, remove_player, remove_staff,
};
use crate::app::teams::io::web::widgets::recruitment_catalog_widget::{
    add_player, add_staff, recruitment_catalog,
};
use crate::app::teams::io::web::widgets::team_match_context_widget::get_team_match_context_json;
use crate::app::teams::io::web::widgets::team_selection_tester::get_team_selection_tester;
use crate::app::teams::io::web::widgets::team_selection_widget::{
    get_team_selection_json, get_team_selection_widget,
};
use crate::app::teams::routes::path;
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::TEAM_DETAIL, get(team_detail))
        .route(path::TEAM_TREASURY, get(team_page_treasury))
        .route(path::DISMISS_TEAM, post(dismiss_team))
        .route(
            path::PENDING_ENROLLMENT_WIDGET,
            get(pending_enrollment_widget),
        )
        .route(path::ENROLLED_TEAMS_WIDGET, get(enrolled_teams_widget))
        .route(path::MY_TEAMS_WIDGET, get(my_teams_widget))
        .route(path::APPROVE_ENROLLMENT, post(approve_enrollment))
        .route(path::REJECT_ENROLLMENT, post(reject_enrollment))
        .route(path::DISMISS_ENROLLMENT, post(dismiss_enrollment))
        .route(path::APPROVE_ALL_ENROLLMENTS, post(approve_all_enrollments))
        .route(
            path::COMPETITION_TEAMS_WIDGET,
            get(competition_teams_widget),
        )
        .route(path::TEAM_SELECTION_WIDGET, get(get_team_selection_widget))
        .route(path::TEAM_SELECTION_JSON, get(get_team_selection_json))
        .route(path::TEAM_SELECTION_TESTER, get(get_team_selection_tester))
        .route(
            path::TEAM_MATCH_CONTEXT_JSON,
            get(get_team_match_context_json),
        )
        .route(
            path::VALIDATE_IMPROVEMENT_PHASE,
            post(post_validate_improvement_phase),
        )
        .route(
            path::VALIDATE_RECRUITMENT_PHASE,
            post(post_validate_recruitment_phase),
        )
        .route(
            path::VALIDATE_DISMISSALS_PHASE,
            post(post_validate_dismissals_phase),
        )
        .route(
            path::COSTLY_MISTAKES_PAGE,
            get(crate::app::teams::io::web::costly_mistakes::get_costly_mistakes_page),
        )
        .route(
            path::COSTLY_MISTAKES_ROLL,
            post(crate::app::teams::io::web::costly_mistakes::post_costly_mistakes_roll),
        )
        .route(path::RECRUITMENT_PAGE, get(recruitment_page))
        .route(path::RECRUITMENT_CATALOG_WIDGET, get(recruitment_catalog))
        .route(path::RECRUITMENT_CART_WIDGET, get(recruitment_cart))
        .route(path::RECRUITMENT_ADD_PLAYER, post(add_player))
        .route(path::RECRUITMENT_REMOVE_PLAYER, post(remove_player))
        .route(path::RECRUITMENT_ADD_STAFF, post(add_staff))
        .route(path::RECRUITMENT_REMOVE_STAFF, post(remove_staff))
        .route(path::DISMISSALS_PAGE, get(dismissals_page))
        .route(path::DISMISSALS_ROSTER_WIDGET, get(dismissals_roster))
        .route(path::DISMISSALS_CART_WIDGET, get(dismissals_cart))
        .route(path::DISMISSALS_MARK_PLAYER, post(mark_player))
        .route(path::DISMISSALS_UNMARK_PLAYER, post(unmark_player))
        .route(path::DISMISSALS_MARK_STAFF, post(mark_staff))
        .route(path::DISMISSALS_UNMARK_STAFF, post(unmark_staff))
}
