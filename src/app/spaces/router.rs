use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::all_spaces::space_all;
use crate::app::spaces::io::web::controllers::member_actions::{
    add_member_controller, change_member_role_controller, remove_member_controller,
};
use crate::app::spaces::io::web::controllers::space_admin_controller::space_admin_controller;
use crate::app::spaces::io::web::controllers::widgets::coach_search::search_coaches_controller;
use crate::app::spaces::io::web::controllers::widgets::coach_search_results::coaches_search_results_controller;
use crate::app::spaces::io::web::controllers::widgets::coach_select::get_coach_selector_widget;
use crate::app::spaces::io::web::controllers::widgets::space_admin_candidates_widget::space_admin_candidates_widget;
use crate::app::spaces::io::web::controllers::widgets::space_admin_members_widget::space_admin_members_widget;
use crate::app::spaces::io::web::controllers::widgets::spaces_sidebar_widget::get_spaces_sidebar;
use crate::app::spaces::io::web::join_spaces::join_spaces;
use crate::app::spaces::io::web::register_space::{register_space, register_space_submit};
use crate::app::spaces::routes::path;
use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;

/// Générique sur l'état de l'application hôte — cf. `auth::router`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    SpacesContext: FromRef<S>,
{
    Router::new()
        .route(
            path::NEW_SPACE,
            get(register_space).post(register_space_submit),
        )
        .route(path::SPACE_ALL, get(space_all))
        .route(path::SPACE_ADMIN, get(space_admin_controller))
        .route(
            path::SPACE_ADMIN_MEMBERS_WIDGET,
            get(space_admin_members_widget),
        )
        .route(
            path::SPACE_ADMIN_CANDIDATES_WIDGET,
            get(space_admin_candidates_widget),
        )
        .route(path::SPACE_ADMIN_MEMBER_ADD, post(add_member_controller))
        .route(
            path::SPACE_ADMIN_MEMBER_ROLE,
            post(change_member_role_controller),
        )
        .route(
            path::SPACE_ADMIN_MEMBER_REMOVE,
            post(remove_member_controller),
        )
        .route(path::SPACES_SIDEBAR, get(get_spaces_sidebar))
        .route(path::SPACE_JOIN, post(join_spaces))
        .route(path::COACH_SELECT_WIDGET, get(get_coach_selector_widget))
        .route(path::COACH_SEARCH_WIDGET, get(search_coaches_controller))
        .route(
            path::COACH_SEARCH_RESULT,
            get(coaches_search_results_controller),
        )
}
