use crate::app::match_report::io::web::actions_step_controller::{get_step3, get_step4};
use crate::app::match_report::io::web::step5_controller::{get_step5, post_step5};
use crate::app::match_report::io::web::inducements_controller::{get_inducements, post_inducements};
use crate::app::match_report::io::web::match_selection_controller::{
    create_match_report, edit_match_report, from_pairing, new_match_report,
    update_match_selection,
};
use crate::app::match_report::io::web::pre_match_controller::{get_pre_match, post_pre_match};
use crate::app::match_report::io::web::recap_controller::{get_recap, post_publish};
use crate::app::match_report::io::web::record_action_controller::{
    delete_action, post_action_step3, post_action_step4,
};
use crate::app::match_report::io::web::widgets::action_log_widget::{
    get_action_log_step3, get_action_log_step4,
};
use crate::app::match_report::io::web::widgets::action_panel_widget::{
    get_action_panel_step3, get_action_panel_step4,
};
use crate::app::match_report::io::web::widgets::turn_selector_widget::{
    get_turn_selector_step3, get_turn_selector_step4,
};
use crate::app::match_report::io::web::widgets::mercenary_selector_widget::get_mercenary_selector;
use crate::app::match_report::io::web::widgets::temp_player_selector_widget::{
    get_temp_players_step3, get_temp_players_step4,
};
use crate::app::match_report::routes::path;
use crate::state::AppState;
use axum::{
    routing::{delete, get, post},
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
        .route(path::MATCH_REPORT_STEP5, get(get_step5).post(post_step5))
        .route(path::MATCH_REPORT_STEP3_TURN_SELECTOR, get(get_turn_selector_step3))
        .route(path::MATCH_REPORT_STEP4_TURN_SELECTOR, get(get_turn_selector_step4))
        .route(path::MATCH_REPORT_STEP3_TEMP_PLAYERS, get(get_temp_players_step3))
        .route(path::MATCH_REPORT_STEP4_TEMP_PLAYERS, get(get_temp_players_step4))
        .route(path::MATCH_REPORT_STEP3_ACTION_PANEL, get(get_action_panel_step3))
        .route(path::MATCH_REPORT_STEP4_ACTION_PANEL, get(get_action_panel_step4))
        .route(path::MATCH_REPORT_STEP3_LOG, get(get_action_log_step3))
        .route(path::MATCH_REPORT_STEP4_LOG, get(get_action_log_step4))
        .route(path::MATCH_REPORT_STEP3_ACTIONS, post(post_action_step3))
        .route(path::MATCH_REPORT_STEP4_ACTIONS, post(post_action_step4))
        .route(path::MATCH_REPORT_ACTION, delete(delete_action))
        .route(path::MATCH_REPORT_MERCENARY_SELECTOR, get(get_mercenary_selector))
        .route(path::MATCH_REPORT_RECAP, get(get_recap))
        .route(path::MATCH_REPORT_RECAP_PUBLISH, post(post_publish))
}
