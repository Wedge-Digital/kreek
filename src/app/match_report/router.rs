use crate::app::match_report::io::web::match_selection_controller::{
    create_match_report, edit_match_report, from_pairing, new_match_report,
    teams_fragment, update_match_selection,
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
        .route(path::MATCH_REPORT_TEAMS, get(teams_fragment))
        .route(path::MATCH_REPORT_FROM_PAIRING, get(from_pairing))
}
