use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::{render_admin_page, require_admin_access};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/groups.html")]
pub struct GroupsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}

impl IntoResponse for GroupsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("groups tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn groups_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        if let Err(resp) = require_admin_access(&auth_session, &space_id, &competition_id, &state).await {
            return resp;
        }

        return GroupsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
        }
        .into_response();
    }

    render_admin_page(auth_session, &space_id, &competition_id, &season_id, "groups", &state).await
}
