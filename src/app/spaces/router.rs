use axum::routing::{get, post};
use axum::Router;
use crate::app::spaces::io::web::join_spaces::join_spaces;
use crate::app::spaces::io::web::register_space::{register_space, register_space_submit};
use crate::app::spaces::io::web::all_spaces::space_all;
use crate::app::spaces::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::NEW_SPACE,      get(register_space).post(register_space_submit))
        .route(path::SPACE_ALL,      get(space_all))
        .route(path::SPACE_JOIN,     post(join_spaces))
}