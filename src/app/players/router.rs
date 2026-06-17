use crate::app::players::io::web::player_table::player_table_widget;
use crate::app::players::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route(path::PLAYERS_BY_TEAM_WIDGET, get(player_table_widget))
}
