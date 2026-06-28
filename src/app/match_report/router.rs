use crate::app::match_report::io::web::actions_step_controller::{get_step3, get_step4};
use crate::app::match_report::io::web::inducements_controller::{get_inducements, post_inducements};
use crate::app::match_report::io::web::match_selection_controller::{
    create_match_report, edit_match_report, from_pairing, new_match_report,
    update_match_selection,
};
use crate::app::match_report::io::web::pre_match_controller::{get_pre_match, post_pre_match};
use crate::app::match_report::io::web::widgets::turn_selector_widget::{
    get_turn_selector_step3, get_turn_selector_step4,
};
use crate::app::match_report::io::web::widgets::temp_player_selector_widget::{
    get_temp_players_step3, get_temp_players_step4,
};
use crate::app::match_report::routes::path;
use crate::state::AppState;
use axum::{
    routing::get,
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::MATCH_REPORT_NEW, get(new_match_report).post(create_match_report))
        .route(path::MATCH_REPORT_EDIT, get(edit_match_report).post(update_match_selection))
        .route(path::MATCH_REPORT_FROM_PAIRING, get(from_pairing))
        .route(path::MATCH_REPORT_STEP2, get(get_pre_match).post(post_pre_match))
        .route(path::MATCH_REPORT_INDUCEMENTS, get(get_inducements).post(post_inducements))
        .route(path::MATCH_REPORT_STEP3, get(get_step3))
        .route(path::MATCH_REPORT_STEP4, get(get_step4))
        .route(path::MATCH_REPORT_STEP3_TURN_SELECTOR, get(get_turn_selector_step3))
        .route(path::MATCH_REPORT_STEP4_TURN_SELECTOR, get(get_turn_selector_step4))
        .route(path::MATCH_REPORT_STEP3_TEMP_PLAYERS, get(get_temp_players_step3))
        .route(path::MATCH_REPORT_STEP4_TEMP_PLAYERS, get(get_temp_players_step4))
}
