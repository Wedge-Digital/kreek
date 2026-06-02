use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "team-detail.html")]
pub struct TeamDetailTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: TeamCreationRoutes,
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
        web_routes: Default::default(),
        team_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}
