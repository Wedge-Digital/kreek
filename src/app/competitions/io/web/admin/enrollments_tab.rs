use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::{
    render_admin_page, require_admin_access,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/enrollments.html")]
pub struct EnrollmentsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub requires_validation: bool,
}

impl IntoResponse for EnrollmentsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("enrollments tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn enrollments_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        if let Err(resp) = require_admin_access(
            &auth_session,
            &space_id,
            &competition_id,
            &season_id,
            &state,
        )
        .await
        {
            return resp;
        }

        let requires_validation = match SeasonId::try_new(&season_id) {
            Ok(sid) => state
                .competitions
                .season_repository
                .find_invitations(&sid)
                .await
                .ok()
                .flatten()
                .map(|inv| inv.requires_validation.0)
                .unwrap_or(true),
            Err(_) => true,
        };

        return EnrollmentsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
            requires_validation,
        }
        .into_response();
    }

    render_admin_page(
        auth_session,
        &space_id,
        &competition_id,
        &season_id,
        "enrollments",
        &state,
    )
    .await
}
