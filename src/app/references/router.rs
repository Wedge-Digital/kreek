use crate::app::references::io::web::league_selector::league_selector;
use crate::app::references::io::web::skill_picker::skill_picker;
use crate::app::references::io::web::special_rule_selector::special_rule_selector;
use crate::app::references::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};
use crate::app::references::io::web::inducement_picker_controller::inducement_picker_controller;
use crate::app::references::io::web::ref_widget_tester_controller::widget_tester_controller;
use crate::app::references::io::web::roster_picker_controller::roster_picker_controller;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::LEAGUE_SELECTOR, get(league_selector))
        .route(path::SPECIAL_RULE_SELECTOR, get(special_rule_selector))
        .route(path::SKILL_PICKER, get(skill_picker))
        .route(path::REF_WIDGETS, get(widget_tester_controller))
        .route(path::ROSTER_PICKER, get(roster_picker_controller))
        .route(path::INDUCEMENT_PICKER, get(inducement_picker_controller))
}
