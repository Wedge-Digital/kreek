use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
use crate::app::competitions::use_cases::admin::{generate_all_pairings, generate_pairings};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn schedule_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "scheduleChanged")
        .body(Body::empty())
        .unwrap()
}

// ── Generate all pairings ────────────────────────────────────────────────────

pub async fn post_generate_all(
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    match generate_all_pairings::execute(
        &season_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.group_repository.as_ref(),
    )
    .await
    {
        Ok(()) => schedule_changed(),
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
    match state
        .competitions
        .match_day_repository
        .clear_all_pairings(&season_id)
        .await
    {
        Ok(()) => schedule_changed(),
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
    let name = body
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Journée {}", position + 1));

    let match_day = MatchDay {
        id: ulid::Ulid::new().to_string(),
        season_id: season_id.clone(),
        name,
        day_type: MatchDayType::FixedDate,
        date_start: None,
        date_end: None,
        position,
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
    let name = body
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Repos {}", position + 1));

    let match_day = MatchDay {
        id: ulid::Ulid::new().to_string(),
        season_id: season_id.clone(),
        name,
        day_type: MatchDayType::Rest,
        date_start: None,
        date_end: None,
        position,
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

    let updated = MatchDay {
        name: body.name.unwrap_or(existing.name),
        day_type: body
            .day_type
            .as_deref()
            .map(MatchDayType::from_str)
            .unwrap_or(existing.day_type),
        date_start: body.date_start.or(existing.date_start),
        date_end: body.date_end.or(existing.date_end),
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
    match state
        .competitions
        .match_day_repository
        .delete_match_day(&round_id)
        .await
    {
        Ok(()) => schedule_changed(),
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
    Path((_space_id, _competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RoundIdBody>,
) -> Response {
    match generate_pairings::execute(
        &body.round_id,
        &season_id,
        state.competitions.match_day_repository.as_ref(),
        state.competitions.group_repository.as_ref(),
    )
    .await
    {
        Ok(()) => schedule_changed(),
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
    match state
        .competitions
        .match_day_repository
        .clear_pairings(&body.round_id)
        .await
    {
        Ok(()) => schedule_changed(),
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
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AddMatchBody>,
) -> Response {
    let pairing = Pairing {
        id: ulid::Ulid::new().to_string(),
        home_team_id: body.home_team_id,
        away_team_id: body.away_team_id,
    };

    match state
        .competitions
        .match_day_repository
        .save_pairing(&body.round_id, &pairing)
        .await
    {
        Ok(()) => schedule_changed(),
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

pub async fn delete_match(
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<DeleteMatchBody>,
) -> Response {
    match state
        .competitions
        .match_day_repository
        .delete_pairing(&body.pairing_id)
        .await
    {
        Ok(()) => schedule_changed(),
        Err(e) => {
            tracing::error!("delete_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
