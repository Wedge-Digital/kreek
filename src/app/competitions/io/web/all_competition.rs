use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::competitions::domain::competition_repository_port::CompetitionSummary;
use crate::app::competitions::routes::Routes;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

#[derive(Template)]
#[template(path = "all-competitions.html")]
pub struct AllCompetitionTemplate {
    pub competition_routes: Routes,
    pub space_id:           String,
    pub competitions:       Vec<CompetitionSummary>,
}

impl Default for AllCompetitionTemplate {
    fn default() -> Self {
        Self {
            competition_routes: Routes,
            space_id:           String::new(),
            competitions:       Vec::new(),
        }
    }
}

impl IntoResponse for AllCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_all_competition(
    Path(space_id): Path<String>,
    State(state):   State<AppState>,
    headers:        HeaderMap,
) -> impl IntoResponse {
    let competitions = match SpaceId::try_new(&space_id) {
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        Ok(id) => match state.competitions.competition_repository.find_by_space_id(&id).await {
            Ok(list) => list,
            Err(_)   => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    };

    let tmpl = AllCompetitionTemplate { space_id, competitions, ..Default::default() };

    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}