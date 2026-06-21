use crate::app::teams::io::web::dismiss_team::dismiss_team;
use crate::app::teams::io::web::team_detail::team_detail;
use crate::app::teams::io::web::widgets::enrollment_actions::{
    approve_all_enrollments, approve_enrollment, dismiss_enrollment, reject_enrollment,
};
use crate::app::teams::io::web::widgets::enrolled_teams_widget::enrolled_teams_widget;
use crate::app::teams::io::web::widgets::pending_enrollment_widget::pending_enrollment_widget;
use crate::app::teams::routes::path;
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::TEAM_DETAIL, get(team_detail))
        .route(path::DISMISS_TEAM, post(dismiss_team))
        .route(
            path::PENDING_ENROLLMENT_WIDGET,
            get(pending_enrollment_widget),
        )
        .route(path::ENROLLED_TEAMS_WIDGET, get(enrolled_teams_widget))
        .route(path::APPROVE_ENROLLMENT, post(approve_enrollment))
        .route(path::REJECT_ENROLLMENT, post(reject_enrollment))
        .route(path::DISMISS_ENROLLMENT, post(dismiss_enrollment))
        .route(path::APPROVE_ALL_ENROLLMENTS, post(approve_all_enrollments))
}
