use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::common_types::{EntityId, SpaceId, UserId};
use crate::app::teams::use_cases::commands::DismissTeamCommand;
use crate::app::teams::use_cases::dismiss_team as dismiss_uc;
use crate::state::AppState;
use crate::web::extractors::space_permissions::SpacePermissions;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn dismiss_team(
    Path((space_id, team_id)): Path<(String, String)>,
    perms: SpacePermissions,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }

    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let space_id_val = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = DismissTeamCommand {
        team_id: team_id_val,
        space_id: space_id_val,
        admin_id: UserId::try_new(&user.id.to_string()).unwrap(),
    };

    match dismiss_uc::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => {}
        Err(dismiss_uc::DismissTeamError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(dismiss_uc::DismissTeamError::Domain(e)) => {
            tracing::warn!("dismiss_team domaine: {e}");
            return StatusCode::UNPROCESSABLE_ENTITY.into_response();
        }
        Err(dismiss_uc::DismissTeamError::Repository(e)) => {
            tracing::error!("dismiss_team repo: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    Response::builder()
        .header("HX-Refresh", "true")
        .body(Body::empty())
        .unwrap()
}
