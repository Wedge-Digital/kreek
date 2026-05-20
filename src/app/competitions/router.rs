use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use crate::app::competitions::io::web::all_competition::get_all_competition;
use crate::app::competitions::io::web::new_competition::{get_new_competition_phase_1, get_members_widget};
use crate::app::competitions::routes::path;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST,        get(get_all_competition))
        .route(path::COMPETITION_NEW,         get(get_new_competition_phase_1))
        .route(path::COMPETITION_NEW_MEMBERS, get(get_members_widget))
}