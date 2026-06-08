use crate::app::players::io::web::player_table::player_table_widget;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/app/{space_id}/players/by-team/{team_id}/widget",
            get(player_table_widget),
        )
}
