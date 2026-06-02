use crate::app::references::io::web::league_selector::league_selector;
use crate::app::references::io::web::skill_picker::skill_picker;
use crate::app::references::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::LEAGUE_SELECTOR, get(league_selector))
        .route(path::SKILL_PICKER, get(skill_picker))
}
