use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::ISpaceUserCacheRepository;
use crate::app::spaces::routes::Routes;
use crate::state::AppState;

// ── Widget (barre de recherche + panneau résultats) ──────────────────────────

#[derive(Template)]
#[template(path = "space-coach-search-widget.html")]
pub struct SpaceCoachSearchWidgetTemplate {
    pub search_url: String,
}

impl IntoResponse for SpaceCoachSearchWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_coach_search_widget(
    Path(space_id): Path<String>,
) -> impl IntoResponse {
    SpaceCoachSearchWidgetTemplate {
        search_url: Routes.coach_search(&space_id),
    }
}

// ── Résultats de recherche ────────────────────────────────────────────────────

pub struct CoachResultItem {
    pub id:         String,
    pub coach_name: String,
    pub initials:   String,
}

#[derive(Template)]
#[template(path = "space-coach-search-results.html")]
pub struct CoachSearchResultsTemplate {
    pub coaches: Vec<CoachResultItem>,
}

impl IntoResponse for CoachSearchResultsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q:        String,
    #[serde(default)]
    pub excluded: String,
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

pub async fn search_coaches(
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let excluded: std::collections::HashSet<String> = params.excluded
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let query = params.q.trim().to_lowercase();

    let all_members = state.spaces
        .user_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let coaches = all_members
        .into_iter()
        .filter_map(|u| {
            let id   = u.id.to_string();
            if excluded.contains(&id) { return None; }
            let name = u.name.into_inner();
            if !query.is_empty() && !name.to_lowercase().contains(&query) { return None; }
            let ini  = initials(&name);
            Some(CoachResultItem { id, coach_name: name, initials: ini })
        })
        .collect();

    CoachSearchResultsTemplate { coaches }.into_response()
}