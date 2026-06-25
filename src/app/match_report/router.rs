use crate::app::match_report::io::web::match_selection_controller::{
    create_match_report, edit_match_report, from_pairing, new_match_report,
    update_match_selection,
};
use crate::app::match_report::io::web::pre_match_controller::{get_pre_match, post_pre_match};
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
}
