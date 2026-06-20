use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::team_creation::use_cases::cancel_creation_spp::{self, CancelCreationSppCommand};
use crate::app::team_creation::use_cases::spend_creation_spp::{self, SpendCreationSppCommand};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn spp_error(msg: &str) -> Response {
    Response::builder()
        .status(422)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<div class="spp-error">{msg}</div>"#)))
        .unwrap()
}

fn skills_updated_trigger(team: &RosterSelectedTeam, player_id: &str, space_id: &str, team_id: &str) -> String {
    let player = team.hired_players().iter().find(|p| p.instance_id.0 == player_id);
    let roster_line_id = player.map(|p| p.definition.id.0.as_str()).unwrap_or("");
    let acquired: Vec<&str> = player
        .map(|p| p.acquired_skills.iter().map(|a| a.skill_id.0.as_str()).collect())
        .unwrap_or_default();
    let acquired_csv = acquired.join(",");
    let routes = TeamCreationRoutes;

    serde_json::json!({
        "skillsUpdated": {
            "player_id": player_id,
            "roster_line_id": roster_line_id,
            "spp": team.spp_pool,
            "acquired": acquired_csv,
            "on_acquire": routes.spend_spp(space_id, team_id, player_id),
            "on_cancel": routes.spend_spp(space_id, team_id, player_id)
        }
    })
    .to_string()
}

// ── Handler POST spend SPP ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SpendSppBody {
    pub skill_id: String,
    pub mode: String,
}

pub async fn spend_spp(
    Path((space_id, team_id, instance_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SpendSppBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mode = match body.mode.as_str() {
        "random" => AcquisitionMode::Random,
        _ => AcquisitionMode::Chosen,
    };

    let cmd = SpendCreationSppCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        skill_id: SkillId(body.skill_id.clone()),
        mode,
    };

    let team = match spend_creation_spp::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        state.team_creation.reference_data.as_ref(),
    )
    .await
    {
        Ok(t) => t,
        Err(spend_creation_spp::SpendSppError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(spend_creation_spp::SpendSppError::PlayerNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(spend_creation_spp::SpendSppError::SkillCostNotFound) => {
            return spp_error("Compétence ou position introuvable dans le référentiel.")
        }
        Err(spend_creation_spp::SpendSppError::Domain(e)) => {
            return spp_error(&e.to_string())
        }
        Err(spend_creation_spp::SpendSppError::Repository(e)) => {
            tracing::error!("spend_spp repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let trigger = skills_updated_trigger(&team, &instance_id, &space_id, &team_id);

    Response::builder()
        .header("HX-Trigger", trigger)
        .body(Body::empty())
        .unwrap()
        .into_response()
}

// ── Handler DELETE cancel SPP ────────────────────────────────────────────────

pub async fn cancel_spp(
    Path((space_id, team_id, instance_id, skill_id)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = CancelCreationSppCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        skill_id: SkillId(skill_id.clone()),
    };

    let team = match cancel_creation_spp::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
    )
    .await
    {
        Ok(t) => t,
        Err(cancel_creation_spp::CancelSppError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(cancel_creation_spp::CancelSppError::Domain(e)) => {
            return spp_error(&e.to_string())
        }
        Err(cancel_creation_spp::CancelSppError::Repository(e)) => {
            tracing::error!("cancel_spp repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let trigger = skills_updated_trigger(&team, &instance_id, &space_id, &team_id);

    Response::builder()
        .header("HX-Trigger", trigger)
        .body(Body::empty())
        .unwrap()
        .into_response()
}
