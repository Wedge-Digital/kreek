use askama::Template;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes:  WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
}

impl IntoResponse for BuildTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn build_team(
    Path((space_id, team_id)): Path<(String, String)>,
) -> impl IntoResponse {
    BuildTeamTemplate {
        web_routes:  Default::default(),
        team_routes: Default::default(),
        space_id,
        team_id,
    }.into_response()
}