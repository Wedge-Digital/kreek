use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "team-detail.html")]
pub struct TeamDetailTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
}

impl IntoResponse for TeamDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn team_detail(
    Path((space_id, team_id)): Path<(String, String)>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    TeamDetailTemplate {
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}
