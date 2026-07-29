use crate::app::routes::AppRoutes;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "draft-team.html")]
pub struct DraftTeamTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub logo_url_value: String,
    pub logo_error: Option<String>,
    pub is_admin: bool,
}

impl IntoResponse for DraftTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn draft_team(perms: SpacePermissions) -> impl IntoResponse {
    DraftTeamTemplate {
        app_routes: Default::default(),
        space_id: perms.space_id.to_string(),
        logo_url_value: String::new(),
        logo_error: None,
        is_admin: perms.is_admin(),
    }
    .into_response()
}
