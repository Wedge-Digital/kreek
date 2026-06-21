use crate::app::shared_kernel::common_types::EntityId;
use crate::app::teams::use_cases::approve_enrollment::{self, ApproveEnrollmentError};
use crate::app::teams::use_cases::commands::{ApproveEnrollmentCommand, RejectEnrollmentCommand};
use crate::app::teams::use_cases::reject_enrollment::{self, RejectEnrollmentError};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn enrollment_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "enrollmentChanged")
        .body(Body::empty())
        .unwrap()
}

fn error_response(status: StatusCode) -> Response {
    status.into_response()
}

pub async fn approve_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    let cmd = ApproveEnrollmentCommand {
        team_id: team_entity_id,
        competition_id: String::new(),
        competition_name: String::new(),
        season_id: String::new(),
        season_name: String::new(),
    };

    match approve_enrollment::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(ApproveEnrollmentError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(ApproveEnrollmentError::Domain(_)) => error_response(StatusCode::UNPROCESSABLE_ENTITY),
        Err(ApproveEnrollmentError::Repository(e)) => {
            tracing::error!("approve_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn reject_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    let cmd = RejectEnrollmentCommand {
        team_id: team_entity_id,
    };

    match reject_enrollment::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(RejectEnrollmentError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(RejectEnrollmentError::Domain(_)) => error_response(StatusCode::UNPROCESSABLE_ENTITY),
        Err(RejectEnrollmentError::Repository(e)) => {
            tracing::error!("reject_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn dismiss_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    use crate::app::teams::use_cases::commands::DismissTeamCommand;
    use crate::app::teams::use_cases::dismiss_team;

    let cmd = DismissTeamCommand {
        team_id: team_entity_id.clone(),
        space_id: EntityId::new(),
        admin_id: EntityId::new(),
    };

    match dismiss_team::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(dismiss_team::DismissTeamError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(dismiss_team::DismissTeamError::Domain(_)) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY)
        }
        Err(dismiss_team::DismissTeamError::Repository(e)) => {
            tracing::error!("dismiss_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct ApproveAllParams {
    pub competition_id: String,
    pub season_id: String,
}

pub async fn approve_all_enrollments(
    Query(params): Query<ApproveAllParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let pending = match state
        .teams
        .team_repository
        .find_by_season_and_status(&params.season_id, "PendingEnrollment")
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("approve_all_enrollments: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    for row in pending {
        let team_id = match EntityId::try_new(&row.team_id) {
            Ok(id) => id,
            Err(_) => continue,
        };
        let cmd = ApproveEnrollmentCommand {
            team_id,
            competition_id: params.competition_id.clone(),
            competition_name: String::new(),
            season_id: params.season_id.clone(),
            season_name: String::new(),
        };
        if let Err(e) = approve_enrollment::execute(cmd, state.teams.team_repository.as_ref()).await
        {
            tracing::warn!("approve_all skip {}: {e:?}", row.team_id);
        }
    }

    enrollment_changed()
}
