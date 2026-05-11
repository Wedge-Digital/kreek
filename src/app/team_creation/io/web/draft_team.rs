use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::team_creation::routes::Routes;
use crate::web::app_layout::AppLayout;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template, Default)]
#[template(path = "draft-team.html")]
pub struct DraftTeamTemplate {
    pub routes:          Routes,
    pub logo_url_value:  String,
    pub logo_error:      Option<String>,
}

impl IntoResponse for DraftTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn draft_team(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        DraftTeamTemplate::default().into_response()
    } else {
        let content = DraftTeamTemplate::default().render().unwrap_or_default();
        AppLayout { content, routes:WebRoutes }.into_response()
    }
}
