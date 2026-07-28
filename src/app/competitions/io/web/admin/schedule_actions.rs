use crate::app::competitions::domain::match_day::{MatchDay, MatchDayName, MatchDayPosition, MatchDayType};
use crate::app::competitions::use_cases::admin::{
    add_match_use_case, delete_pairing_use_case, generate_all_pairings, generate_pairings,
};
use crate::app::shared_kernel::common_types::{MatchId, SeasonId};
use crate::app::shared_kernel::date_string::DateString;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

fn schedule_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "scheduleChanged")
        .body(Body::empty())
        .unwrap()
}

#[derive(Serialize, Default)]
struct ScheduleActionResult {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_teams: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_rounds: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_groups: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_matches: Vec<String>,
}

/// Suppression en masse : les rencontres à rapport publié sont conservées, le
/// reste est supprimé. L'admin doit savoir lesquelles ont résisté.
fn schedule_changed_with_kept(kept: Vec<delete_pairing_use_case::KeptMatch>) -> Response {
    let skipped_matches = kept
        .into_iter()
        .map(|m| format!("{} – {} ({})", m.home_team_name, m.away_team_name, m.round_name))
        .collect();

    let mut response = Json(ScheduleActionResult {
        skipped_matches,
        ..Default::default()
    })
    .into_response();
    response.headers_mut().insert("HX-Trigger", "scheduleChanged".parse().unwrap());
    response
}

/// Même effet que `schedule_changed()`, en signalant en plus à l'admin les
/// équipes exclues de l'appariement (non `Enrolled`), les journées non
/// régénérées car elles avaient déjà des appariements, et/ou les poules sans
/// effectif suffisant (< 2 équipes assignées, aucun appariement possible).
fn schedule_changed_with_warning(skipped_teams: Vec<String>, skipped_rounds: Vec<String>, skipped_groups: Vec<String>) -> Response {
    let mut response = Json(ScheduleActionResult {
        skipped_teams,
        skipped_rounds,
        skipped_groups,
        skipped_matches: vec![],
    })
    .into_response();
    response.headers_mut().insert("HX-Trigger", "scheduleChanged".parse().unwrap());
    response
}

#[derive(Serialize)]
struct ErrorResult {
    error: String,
}

fn add_match_refused(names: &[String]) -> Response {
    let message = format!("Match non créé : équipe(s) non enrôlée(s) pour cette saison : {}", names.join(", "));
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResult { error: message })).into_response()
}

fn pairings_already_exist_refused() -> Response {
    let message = "Cette journée a déjà des appariements. Videz-les d'abord (\"Vider les matchs\") avant de régénérer.".to_string();
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResult { error: message })).into_response()
}

// ── Generate all pairings ────────────────────────────────────────────────────

pub async fn post_generate_all(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    match generate_all_pairings::execute(
        &season_id,
        &competition_id,
        &space_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.group_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(outcome) => schedule_changed_with_warning(outcome.skipped_team_names, outcome.skipped_round_names, outcome.skipped_group_names),
        Err(e) => {
            tracing::error!("post_generate_all: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Clear all pairings ───────────────────────────────────────────────────────

pub async fn post_clear_all(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    match delete_pairing_use_case::clear_season(
        &season_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(kept) => schedule_changed_with_kept(kept),
        Err(e) => {
            tracing::error!("post_clear_all: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Add round (match day) ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddRoundBody {
    pub name: Option<String>,
}

pub async fn post_add_round(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AddRoundBody>,
) -> Response {
    let existing = state
        .competitions
        .match_day_repository
        .find_by_season(&season_id)
        .await
        .unwrap_or_default();

    let position = existing.len() as i32;
    let raw_name = body
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Journée {}", position + 1));

    let Ok(sid) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(name) = MatchDayName::try_new(raw_name) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let match_day = MatchDay {
        id: MatchId::new(),
        season_id: sid,
        name,
        day_type: MatchDayType::FixedDate,
        date_start: None,
        date_end: None,
        position: MatchDayPosition::try_new(position).unwrap(),
        pairings: vec![],
    };

    match state
        .competitions
        .match_day_repository
        .save_match_day(&match_day)
        .await
    {
        Ok(()) => schedule_changed(),
        Err(e) => {
            tracing::error!("post_add_round: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Add rest day ─────────────────────────────────────────────────────────────

pub async fn post_add_rest(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AddRoundBody>,
) -> Response {
    let existing = state
        .competitions
        .match_day_repository
        .find_by_season(&season_id)
        .await
        .unwrap_or_default();

    let position = existing.len() as i32;
    let raw_name = body
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Repos {}", position + 1));

    let Ok(sid) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(name) = MatchDayName::try_new(raw_name) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let match_day = MatchDay {
        id: MatchId::new(),
        season_id: sid,
        name,
        day_type: MatchDayType::Rest,
        date_start: None,
        date_end: None,
        position: MatchDayPosition::try_new(position).unwrap(),
        pairings: vec![],
    };

    match state
        .competitions
        .match_day_repository
        .save_match_day(&match_day)
        .await
    {
        Ok(()) => schedule_changed(),
        Err(e) => {
            tracing::error!("post_add_rest: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Update round ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpdateRoundBody {
    pub name: Option<String>,
    pub day_type: Option<String>,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
}

pub async fn put_update_round(
    Path((_space_id, _competition_id, _season_id, round_id)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<UpdateRoundBody>,
) -> Response {
    let existing = match state
        .competitions
        .match_day_repository
        .find_by_id(&round_id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("put_update_round find: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let name = body
        .name
        .and_then(|s| MatchDayName::try_new(s).ok())
        .unwrap_or(existing.name.clone());
    let day_type = body
        .day_type
        .as_deref()
        .map(MatchDayType::from_str)
        .unwrap_or(existing.day_type);
    let date_start = body
        .date_start
        .and_then(|s| DateString::try_new(s).ok())
        .or(existing.date_start);
    let date_end = body
        .date_end
        .and_then(|s| DateString::try_new(s).ok())
        .or(existing.date_end);
    let updated = MatchDay {
        name,
        day_type,
        date_start,
        date_end,
        ..existing
    };

    match state
        .competitions
        .match_day_repository
        .save_match_day(&updated)
        .await
    {
        Ok(()) => schedule_changed(),
        Err(e) => {
            tracing::error!("put_update_round save: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Delete round ─────────────────────────────────────────────────────────────

pub async fn delete_round(
    Path((_space_id, _competition_id, _season_id, round_id)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    match delete_pairing_use_case::delete_round(
        &round_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(kept) => schedule_changed_with_kept(kept),
        Err(e) => {
            tracing::error!("delete_round: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Generate round pairings ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RoundIdBody {
    pub round_id: String,
}

pub async fn post_generate_round_pairings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RoundIdBody>,
) -> Response {
    match generate_pairings::execute(
        &body.round_id,
        &season_id,
        &competition_id,
        &space_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.group_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(outcome) => schedule_changed_with_warning(outcome.skipped_team_names, vec![], outcome.skipped_group_names),
        Err(generate_pairings::GenerateError::PairingsAlreadyExist) => pairings_already_exist_refused(),
        Err(e) => {
            tracing::error!("post_generate_round_pairings: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Clear round pairings ─────────────────────────────────────────────────────

pub async fn post_clear_round_pairings(
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RoundIdBody>,
) -> Response {
    match delete_pairing_use_case::clear_round(
        &body.round_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(kept) => schedule_changed_with_kept(kept),
        Err(e) => {
            tracing::error!("post_clear_round_pairings: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Add match (pairing) ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddMatchBody {
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

pub async fn post_add_match(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AddMatchBody>,
) -> Response {
    match add_match_use_case::execute(
        &body.round_id,
        &season_id,
        &competition_id,
        &space_id,
        &body.home_team_id,
        &body.away_team_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(()) => schedule_changed_with_warning(vec![], vec![], vec![]),
        Err(add_match_use_case::AddMatchError::TeamsNotEnrolled(names)) => add_match_refused(&names),
        Err(add_match_use_case::AddMatchError::RoundNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(add_match_use_case::AddMatchError::InvalidTeamId) => StatusCode::BAD_REQUEST.into_response(),
        Err(e) => {
            tracing::error!("post_add_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Delete match (pairing) ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteMatchBody {
    pub pairing_id: String,
}

fn delete_match_refused() -> Response {
    let message = "Match non supprimé : son rapport est publié. Dépubliez-le depuis son \
                   récapitulatif avant de supprimer la rencontre."
        .to_string();
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResult { error: message })).into_response()
}

pub async fn delete_match(
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<DeleteMatchBody>,
) -> Response {
    match delete_pairing_use_case::execute(
        &body.pairing_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.match_report_status_port.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok(()) => schedule_changed(),
        Err(delete_pairing_use_case::DeletePairingError::ReportPublished) => {
            delete_match_refused()
        }
        Err(e) => {
            tracing::error!("delete_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
