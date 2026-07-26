use crate::app::ranking::io::web::widgets::classement_widget::classement_widget;
use crate::app::ranking::io::web::widgets::detailed_standings_widget::detailed_standings_widget;
use crate::app::ranking::routes::path;
use crate::state::AppState;
use axum::routing::get;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::CLASSEMENT_WIDGET, get(classement_widget))
        .route(path::DETAILED_STANDINGS_WIDGET, get(detailed_standings_widget))
}
