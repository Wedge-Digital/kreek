use crate::app::players::io::web::increase_stat_controller::post_increase_stat;
use crate::app::players::io::web::player_debug_controller::player_debug_controller;
use crate::app::players::io::web::player_detail_controller::player_detail_controller;
use crate::app::players::io::web::purchase_skill_controller::post_purchase_skill;
use crate::app::players::io::web::roster_edition_controller::post_update_roster;
use crate::app::players::io::web::widgets::evolution_journal_widget::evolution_journal_widget;
use crate::app::players::io::web::widgets::match_player_selector_widget::match_player_selector_widget;
use crate::app::players::io::web::widgets::player_table_widget::player_table_widget;
use crate::app::players::io::web::widgets::spp_spending_widget::spp_spending_widget;
use crate::app::players::routes::path;
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::PLAYERS_BY_TEAM_WIDGET, get(player_table_widget))
        .route(path::PLAYERS_ROSTER_UPDATE, post(post_update_roster))
        .route(
            path::MATCH_PLAYER_SELECTOR,
            get(match_player_selector_widget),
        )
        .route(path::PLAYER_DEBUG, get(player_debug_controller))
        .route(path::PLAYER_DETAIL, get(player_detail_controller))
        .route(path::PLAYER_SKILLS, post(post_purchase_skill))
        .route(path::PLAYER_STAT_INCREASE, post(post_increase_stat))
        .route(
            path::PLAYER_EVOLUTION_JOURNAL_WIDGET,
            get(evolution_journal_widget),
        )
        .route(path::PLAYER_SPP_SPENDING_WIDGET, get(spp_spending_widget))
}
