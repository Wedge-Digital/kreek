use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use crate::app::competitions::io::web::all_competition::get_all_competition;
use crate::app::competitions::io::web::new_competition::{get_new_competition_phase_1, get_new_competition_phase_1_edit, get_new_competition_phase_2, post_competition_rules, post_new_competition, post_update_competition};
use crate::app::competitions::io::web::new_competition_phase_3::{get_new_competition_phase_3, post_competition_structure};
use crate::app::competitions::io::web::new_competition_phase_4::{get_new_competition_phase_4, post_competition_invitations};
use crate::app::competitions::io::web::new_competition_phase_5::{get_new_competition_phase_5, post_finalize_competition};
use crate::app::competitions::routes::path;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST,              get(get_all_competition))
        .route(path::COMPETITION_NEW,               get(get_new_competition_phase_1).post(post_new_competition))
        .route(path::COMPETITION_NEW_INFO,          get(get_new_competition_phase_1_edit).post(post_update_competition))
        .route(path::COMPETITION_NEW_RULES,         get(get_new_competition_phase_2).post(post_competition_rules))
        .route(path::COMPETITION_NEW_STRUCTURE,     get(get_new_competition_phase_3).post(post_competition_structure))
        .route(path::COMPETITION_NEW_INVITATIONS, get(get_new_competition_phase_4).post(post_competition_invitations))
        .route(path::COMPETITION_NEW_VALIDATION,  get(get_new_competition_phase_5).post(post_finalize_competition))
}