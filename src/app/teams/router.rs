use axum::{routing::{get, post}, Router};
use crate::app::teams::io::web::dismiss_team::dismiss_team;
use crate::app::teams::io::web::team_detail::team_detail;
use crate::app::teams::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::TEAM_DETAIL,  get(team_detail))
        .route(path::DISMISS_TEAM, post(dismiss_team))
}
