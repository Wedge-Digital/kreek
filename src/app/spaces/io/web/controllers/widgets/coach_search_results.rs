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
#[template(path = "widgets/coach-search-results.html")]
pub struct CoachSearchResultsTemplate {
    pub routes: AppRoutes,
    pub coaches: Vec<CoachDefinition>,
    pub space_id: String,
}

impl IntoResponse for CoachSearchResultsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn coaches_search_results_controller(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&params.space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let excluded: std::collections::HashSet<String> = params
        .excluded
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let query = params.q.trim().to_lowercase();

    let all_members = state
        .spaces
        .user_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let coaches = all_members
        .into_iter()
        .filter_map(|u| {
            let id = u.id.to_string();
            if excluded.contains(&id) {
                return None;
            }
            let name = u.name.into_inner();
            if !query.is_empty() && !name.to_lowercase().contains(&query) {
                return None;
            }
            Some(CoachDefinition {
                id: CoachId::try_new(&id).unwrap(),
                name: CoachName::try_new(&name).unwrap(),
                initials: initials(&name),
                icon: u.icon.clone()
            })
        })
        .collect();

    CoachSearchResultsTemplate {
        routes: AppRoutes::default(),
        coaches,
        space_id: params.space_id,
    }.into_response()
}
