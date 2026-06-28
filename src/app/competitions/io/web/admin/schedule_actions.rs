use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::{
    MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
};
use crate::app::competitions::use_cases::admin::{generate_all_pairings, generate_pairings};
use crate::app::shared_kernel::common_types::{EventId, MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::date_string::DateString;
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn emit_pairing_deleted_events(bus: &EventBus, pairing_ids: &[String]) {
    for pid in pairing_ids {
        let _ = bus.send(
            CompetitionsDomainEvent::PairingDeleted {
                event_id: EventId::new(),
                pairing_id: pid.clone(),
            }
            .to_enveloppe(),
        );
    }
}

fn schedule_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "scheduleChanged")
        .body(Body::empty())
        .unwrap()
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
    let all_days = state
        .competitions
        .match_day_repository
        .find_by_season(&season_id)
        .await
        .unwrap_or_default();
    let pairing_ids: Vec<String> = all_days
        .iter()
        .flat_map(|d| d.pairings.iter().map(|p| p.id.to_string()))
        .collect();

    match state
        .competitions
        .match_day_repository
        .clear_all_pairings(&season_id)
        .await
    {
        Ok(()) => {
            emit_pairing_deleted_events(&state.competitions.event_bus, &pairing_ids);
            schedule_changed()
        }
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
    let match_day = state
        .competitions
        .match_day_repository
        .find_by_id(&round_id)
        .await
        .ok()
        .flatten();
    let pairing_ids: Vec<String> = match_day
        .iter()
        .flat_map(|d| d.pairings.iter().map(|p| p.id.to_string()))
        .collect();

    match state
        .competitions
        .match_day_repository
        .delete_match_day(&round_id)
        .await
    {
        Ok(()) => {
            emit_pairing_deleted_events(&state.competitions.event_bus, &pairing_ids);
            schedule_changed()
        }
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
    let match_day = state
        .competitions
        .match_day_repository
        .find_by_id(&body.round_id)
        .await
        .ok()
        .flatten();
    let pairing_ids: Vec<String> = match_day
        .iter()
        .flat_map(|d| d.pairings.iter().map(|p| p.id.to_string()))
        .collect();

    match state
        .competitions
        .match_day_repository
        .clear_pairings(&body.round_id)
        .await
    {
        Ok(()) => {
            emit_pairing_deleted_events(&state.competitions.event_bus, &pairing_ids);
            schedule_changed()
        }
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
    let Ok(home) = TeamId::try_new(&body.home_team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(away) = TeamId::try_new(&body.away_team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id: home,
        away_team_id: away,
    };

    match state
        .competitions
        .match_day_repository
        .save_pairing(&body.round_id, &pairing)
        .await
    {
        Ok(()) => {
            let _ = state.competitions.event_bus.send(
                CompetitionsDomainEvent::PairingCreated {
                    event_id: EventId::new(),
                    pairing_id: pairing.id.to_string(),
                    competition_id: competition_id.clone(),
                    season_id: season_id.clone(),
                    round_id: body.round_id.clone(),
                    home_team_id: body.home_team_id,
                    away_team_id: body.away_team_id,
                    space_id: space_id.clone(),
                }
                .to_enveloppe(),
            );
            schedule_changed()
        }
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
        Ok(()) => {
            emit_pairing_deleted_events(&state.competitions.event_bus, &[body.pairing_id]);
            schedule_changed()
        }
        Err(e) => {
            tracing::error!("delete_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
