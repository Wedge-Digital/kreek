use crate::web::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};
use crate::web::app_layout::app_layout;
use crate::web::app_menu::app_menu;
use crate::web::app_spaces::app_spaces;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::APP_LAYOUT,        get(app_layout))
        .route(path::SPACES,            get(app_spaces))
        .route(path::MENU,              get(app_menu))
}