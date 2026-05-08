use axum::{routing::get, Router};
use crate::app::spaces::io::web::register_space::register_space;
use crate::app::spaces::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::NEW_SPACE,        get(register_space))
}