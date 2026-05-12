use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use crate::app::competition::io::web::all_competition::get_all_competition;
use crate::app::competition::routes::path;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST,        get(get_all_competition))
}