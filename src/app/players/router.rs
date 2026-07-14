use crate::app::players::io::web::player_debug_controller::player_debug_controller;
use crate::app::players::io::web::player_detail_controller::player_detail_controller;
use crate::app::players::io::web::player_table::player_table_widget;
use crate::app::players::io::web::widgets::match_player_selector_widget::match_player_selector_widget;
use crate::app::players::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::PLAYERS_BY_TEAM_WIDGET, get(player_table_widget))
        .route(path::MATCH_PLAYER_SELECTOR, get(match_player_selector_widget))
        .route(path::PLAYER_DEBUG, get(player_debug_controller))
        .route(path::PLAYER_DETAIL, get(player_detail_controller))
}
