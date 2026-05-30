use axum::{routing::{get, post}, Router};
use crate::app::team_creation::io::web::build_team::build_team;
use crate::app::team_creation::io::web::draft_team::draft_team;
use crate::app::team_creation::io::web::post_draft_team::post_draft_team;
use crate::app::team_creation::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::DRAFT_TEAM, get(draft_team).post(post_draft_team))
        .route(path::TEAM_BUILD, get(build_team))
}