use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::team_creation::routes::Routes;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;
use crate::web::extractors::space_permissions::SpacePermissions;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template, Default)]
#[template(path = "draft-team.html")]
pub struct DraftTeamTemplate {
    pub routes:          Routes,
    pub logo_url_value:  String,
    pub logo_error:      Option<String>,
    pub is_admin:        bool,
}

impl IntoResponse for DraftTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn draft_team(
    perms: SpacePermissions,
    headers: HeaderMap,
) -> impl IntoResponse {
    let tmpl = DraftTeamTemplate { is_admin: perms.is_admin(), ..Default::default() };

    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: WebRoutes }.into_response()
    }
}
