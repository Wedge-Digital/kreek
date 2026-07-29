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
    let team = match state.teams.team_repository.find_by_id(&query.team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // `ISquadPort` a remplacé le port de comptage : on charge l'effectif pour
    // en prendre la taille. Un `COUNT(*)` était plus direct, mais deux ports
    // vers la même source coûtaient plus cher à maintenir que ce chargement —
    // seize joueurs au plus.
    let player_count = state
        .teams
        .squad_port
        .find_squad(&query.team_id)
        .await
        .len() as u32;

    let journeyman_type = state
        .teams
        .journeyman_type_port
        .journeyman_type_for_roster(&team.roster_id.to_string())
        .position_name;

    Json(TeamMatchContextJson {
        team_id: team.id.to_string(),
        team_name: team.name.to_string(),
        coach_name: team.coach_name.clone(),
        roster_name: team.roster_name.to_string(),
        dedicated_fans: team.dedicated_fans.into_inner() as u32,
        player_count,
        ctv: team.team_value.0,
        treasury: team.treasury.0,
        journeyman_type,
    })
    .into_response()
}
