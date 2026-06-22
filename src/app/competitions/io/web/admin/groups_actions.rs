use crate::app::competitions::use_cases::admin::{assign_team_to_group, random_draw, reset_groups};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn groups_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "groupsChanged")
        .body(Body::empty())
        .unwrap()
}

pub async fn post_random_draw(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    match random_draw::execute(
        &season_id,
        state.competitions.group_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
    )
    .await
    {
        Ok(()) => groups_changed(),
        Err(random_draw::DrawError::NoGroups) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(random_draw::DrawError::NoTeams) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(e) => {
            tracing::error!("random_draw: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn post_reset_groups(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match reset_groups::execute(&season_id, state.competitions.group_repository.as_ref()).await {
        Ok(()) => groups_changed(),
        Err(e) => {
            tracing::error!("reset_groups: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct AssignBody {
    pub team_id: String,
    pub group_id: String,
}

pub async fn post_assign_team(
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AssignBody>,
) -> impl IntoResponse {
    match assign_team_to_group::execute(
        &body.team_id,
        &body.group_id,
        state.competitions.group_repository.as_ref(),
    )
    .await
    {
        Ok(()) => groups_changed(),
        Err(e) => {
            tracing::error!("assign_team: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
