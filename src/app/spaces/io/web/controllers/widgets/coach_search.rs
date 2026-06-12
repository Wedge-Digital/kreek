use crate::app::shared_kernel::common_types::{CoachId, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::coach_definition::CoachDefinition;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::common::initials::initials;

// ── Résultats de recherche ────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct SearchParams {
    pub space_id: String,
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub excluded: String,
}

#[derive(Template)]
#[template(path = "widgets/coach-search.html")]
pub struct CoachSearchTemplate {
    pub routes: AppRoutes,
    pub space_id: String,
}

impl IntoResponse for CoachSearchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn search_coaches_controller(
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    CoachSearchTemplate {
        routes: AppRoutes::default(),
        space_id: params.space_id,
    }.into_response()
}