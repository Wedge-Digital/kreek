use axum::{routing::get, Router};
use crate::app::teams::io::web::team_detail::team_detail;
use crate::app::teams::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::TEAM_DETAIL, get(team_detail))
}
