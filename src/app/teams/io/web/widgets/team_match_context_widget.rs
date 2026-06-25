use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TeamMatchContextJson {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub dedicated_fans: u32,
    pub player_count: u32,
    pub ctv: u32,
    pub treasury: u32,
    pub journeyman_type: String,
}

#[derive(Deserialize)]
pub struct TeamMatchContextQuery {
    pub team_id: String,
}

pub async fn get_team_match_context_json(
    Path(_space_id): Path<String>,
    Query(query): Query<TeamMatchContextQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = match state
        .teams
        .team_repository
        .find_by_id(&query.team_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let player_count = state
        .teams
        .player_count_port
        .count_for_team(&query.team_id)
        .await;

    let journeyman_type = state
        .teams
        .journeyman_type_port
        .journeyman_type_for_roster(&team.roster_id);

    Json(TeamMatchContextJson {
        team_id: team.id.clone(),
        team_name: team.name.clone(),
        coach_name: team.coach_name.clone(),
        roster_name: team.roster_name.clone(),
        dedicated_fans: team.dedicated_fans as u32,
        player_count,
        ctv: team.team_value.0,
        treasury: team.treasury.0,
        journeyman_type,
    })
    .into_response()
}
