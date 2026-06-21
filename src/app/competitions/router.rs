use crate::app::competitions::io::web::all_competition::get_all_competition;
use crate::app::competitions::io::web::admin::admin_page::admin_page;
use crate::app::competitions::io::web::admin::dashboard::dashboard_fragment;
use crate::app::competitions::io::web::admin::enrollments_tab::enrollments_tab;
use crate::app::competitions::io::web::competition_detail::{
    get_competition_detail, get_tab_matches, get_tab_standings, get_tab_stats, get_tab_teams,
};
use crate::app::competitions::io::web::competition_widget::{
    get_competition_widget, get_competition_widget_detail, get_competition_widget_seasons,
};
use crate::app::competitions::io::web::new_competition::{
    get_new_competition_phase_1, get_new_competition_phase_1_edit, get_new_competition_phase_2,
    post_competition_rules, post_new_competition, post_update_competition,
};
use crate::app::competitions::io::web::new_competition_phase_3::{
    get_new_competition_phase_3, post_competition_structure,
};
use crate::app::competitions::io::web::new_competition_phase_4::{
    get_new_competition_phase_4, post_competition_invitations,
};
use crate::app::competitions::io::web::new_competition_phase_5::{
    get_new_competition_phase_5, post_finalize_competition,
};
use crate::app::competitions::routes::path;
use crate::state::AppState;
use axum::routing::get;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST, get(get_all_competition))
        .route(path::COMPETITION_DETAIL, get(get_competition_detail))
        .route(
            path::COMPETITION_NEW,
            get(get_new_competition_phase_1).post(post_new_competition),
        )
        .route(
            path::COMPETITION_NEW_INFO,
            get(get_new_competition_phase_1_edit).post(post_update_competition),
        )
        .route(
            path::COMPETITION_NEW_RULES,
            get(get_new_competition_phase_2).post(post_competition_rules),
        )
        .route(
            path::COMPETITION_NEW_STRUCTURE,
            get(get_new_competition_phase_3).post(post_competition_structure),
        )
        .route(
            path::COMPETITION_NEW_INVITATIONS,
            get(get_new_competition_phase_4).post(post_competition_invitations),
        )
        .route(
            path::COMPETITION_NEW_VALIDATION,
            get(get_new_competition_phase_5).post(post_finalize_competition),
        )
        .route(path::COMPETITION_TAB_STANDINGS, get(get_tab_standings))
        .route(path::COMPETITION_TAB_MATCHES, get(get_tab_matches))
        .route(path::COMPETITION_TAB_TEAMS, get(get_tab_teams))
        .route(path::COMPETITION_TAB_STATS, get(get_tab_stats))
        .route(path::COMPETITION_WIDGET, get(get_competition_widget))
        .route(
            path::COMPETITION_WIDGET_SEASONS,
            get(get_competition_widget_seasons),
        )
        .route(
            path::COMPETITION_WIDGET_DETAIL,
            get(get_competition_widget_detail),
        )
        .route(path::COMPETITION_ADMIN, get(admin_page))
        .route(path::COMPETITION_ADMIN_DASHBOARD, get(dashboard_fragment))
        .route(path::COMPETITION_ADMIN_ENROLLMENTS, get(enrollments_tab))
}
