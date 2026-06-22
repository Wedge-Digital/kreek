use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::{CompetitionId, EntityId, SeasonId};
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::app::team_creation::use_cases::submit_team as submit_uc;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn submit_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let auto_enroll = match SeasonId::try_new(draft.season_id()) {
        Ok(sid) => state
            .competitions
            .season_repository
            .find_invitations(&sid)
            .await
            .ok()
            .flatten()
            .map(|inv| !inv.requires_validation)
            .unwrap_or(false),
        Err(_) => false,
    };

    let competition_name = match CompetitionId::try_new(draft.competition_id()) {
        Ok(cid) => state.competitions.competition_repository.find_base_info(&cid).await
            .ok().flatten().map(|i| i.name).unwrap_or_default(),
        Err(_) => String::new(),
    };
    let season_name = match SeasonId::try_new(draft.season_id()) {
        Ok(sid) => state.competitions.season_repository.find_base_info(&sid).await
            .ok().flatten().map(|i| i.name).unwrap_or_default(),
        Err(_) => String::new(),
    };

    let cmd = SubmitTeamCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        competition_id: draft.competition_id().to_string(),
        competition_name,
        season_id: draft.season_id().to_string(),
        season_name,
        coach_name: draft.coach_name().to_string(),
        auto_enroll,
    };

    match submit_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        &state.team_creation.event_bus,
    )
        .await
    {
        Ok(()) => {}
        Err(submit_uc::SubmitTeamError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(submit_uc::SubmitTeamError::Domain(ref errors)) => {
            let msgs: String = errors
                .iter()
                .map(|e| {
                    format!(
                        r#"<p class="table-error">{}</p>"#,
                        submit_uc::domain_error_message(e)
                    )
                })
                .collect();
            return Response::builder()
                .header("HX-Retarget", "#submit-error")
                .header("HX-Reswap", "innerHTML")
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(msgs))
                .unwrap();
        }
        Err(submit_uc::SubmitTeamError::Repository(e)) => {
            tracing::error!("submit_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let app_routes = AppRoutes::default();
    Response::builder()
        .header("HX-Redirect", app_routes.teams.team_detail(&space_id, &team_id))
        .header(
            "HX-Trigger",
            r#"{"showToast":"Équipe soumise avec succès !"}"#,
        )
        .body(Body::empty())
        .unwrap()
}