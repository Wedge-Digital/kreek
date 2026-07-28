use crate::app::spaces::io::web::all_spaces::space_all;
use crate::app::spaces::io::web::join_spaces::join_spaces;
use crate::app::spaces::io::web::widget_tester_controller::get_space_widget_tester;
use crate::app::spaces::io::web::register_space::{register_space, register_space_submit};
use crate::app::spaces::routes::path;
use crate::app::spaces::context::SpacesContext;
use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use crate::app::spaces::io::web::controllers::widgets::coach_search::search_coaches_controller;
use crate::app::spaces::io::web::controllers::widgets::coach_search_results::coaches_search_results_controller;
use crate::app::spaces::io::web::controllers::widgets::coach_select::get_coach_selector_widget;

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
        .route(path::SPACE_JOIN, post(join_spaces))
        .route(path::COACH_SELECT_WIDGET, get(get_coach_selector_widget),
        )
        .route(path::COACH_SEARCH_WIDGET, get(search_coaches_controller),)
        .route(path::COACH_SEARCH_RESULT, get(coaches_search_results_controller),)
        .route(path::SPACE_WIDGET_TESTER, get(get_space_widget_tester))
}
