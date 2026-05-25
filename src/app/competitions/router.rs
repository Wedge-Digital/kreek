use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use crate::app::competitions::io::web::all_competition::get_all_competition;
use crate::app::competitions::io::web::new_competition::{get_members_widget, get_new_competition_phase_1, get_new_competition_phase_2, post_competition_rules, post_new_competition};
use crate::app::competitions::io::web::new_competition_phase_3::{get_new_competition_phase_3, post_competition_structure};
use crate::app::competitions::io::web::new_competition_phase_4::{get_new_competition_phase_4, post_competition_invitations};
use crate::app::competitions::routes::path;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST,              get(get_all_competition))
        .route(path::COMPETITION_NEW,               get(get_new_competition_phase_1).post(post_new_competition))
        .route(path::COMPETITION_NEW_MEMBERS,       get(get_members_widget))
        .route(path::COMPETITION_NEW_RULES,         get(get_new_competition_phase_2).post(post_competition_rules))
        .route(path::COMPETITION_NEW_STRUCTURE,     get(get_new_competition_phase_3).post(post_competition_structure))
        .route(path::COMPETITION_NEW_INVITATIONS, get(get_new_competition_phase_4).post(post_competition_invitations))
}