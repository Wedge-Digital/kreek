use crate::app::ranking::io::web::manual_points::controller::{
    delete_manual_point, manual_points_form, manual_points_list, manual_points_page,
    manual_points_teams_json, post_manual_points,
};
use crate::app::ranking::io::web::widgets::classement_widget::classement_widget;
use crate::app::ranking::io::web::widgets::detailed_standings_widget::detailed_standings_widget;
use crate::app::ranking::routes::path;
use crate::state::AppState;
use axum::routing::{delete, get};
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::CLASSEMENT_WIDGET, get(classement_widget))
        .route(
            path::DETAILED_STANDINGS_WIDGET,
            get(detailed_standings_widget),
        )
        // ── Points manuels (carte 452) ────────────────────────────────────────
        //
        // Les trois `GET` sont ouverts à tout membre : les points manuels sont
        // publics, ils s'affichent déjà dans le classement. Seules les deux
        // mutations demandent d'être commissaire, et ce contrôle vit dans les
        // use cases — pas ici, où il se dupliquerait.
        .route(
            path::MANUAL_POINTS,
            get(manual_points_page).post(post_manual_points),
        )
        .route(path::MANUAL_POINTS_FORM, get(manual_points_form))
        .route(path::MANUAL_POINTS_LIST, get(manual_points_list))
        .route(
            path::MANUAL_POINTS_TEAMS_JSON,
            get(manual_points_teams_json),
        )
        .route(path::MANUAL_POINT, delete(delete_manual_point))
}
