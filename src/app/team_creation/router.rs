use axum::{routing::get, Router};
use crate::app::team_creation::io::web::draft_team::draft_team;
use crate::app::team_creation::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::DRAFT_TEAM,        get(draft_team))
}