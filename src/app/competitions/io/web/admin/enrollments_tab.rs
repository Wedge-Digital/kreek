use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/enrollments.html")]
pub struct EnrollmentsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
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
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let comp_id = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let space_entity_id = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let _season_id = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let is_space_admin = matches!(
        state.spaces.space_repository.find_member_profile(&user.id, &space_entity_id).await,
        Ok(Some(SpaceProfile::SpaceAdmin))
    );

    let comp_info = match state.competitions.competition_repository.find_base_info(&comp_id).await {
        Ok(Some(info)) => info,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("enrollments_tab competition find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let is_comp_admin = comp_info.admin_ids.contains(&user.id.to_string())
        || comp_info.admin_names.contains(&user.coach_name.clone().into_inner());

    if !is_space_admin && !is_comp_admin {
        return StatusCode::FORBIDDEN.into_response();
    }

    EnrollmentsTabTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
    }
    .into_response()
}
