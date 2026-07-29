use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::PlayerId;
use crate::app::players::io::web::purchase_skill_controller::can_spend_spp;
use crate::app::players::use_cases::commands::IncreaseStatCommand;
use crate::app::players::use_cases::increase_stat_use_case;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn post_increase_stat(
    Path((space_id, player_id, stat)): Path<(String, String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Some(stat) = parse_stat(&stat) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let player = match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.clone()))
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("post_increase_stat find player {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Some(team) = state
        .players
        .roster_port
        .find_team_info(&player.team_id.0)
        .await
    else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let space_id_vo = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if !team.in_player_improvement_phase {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !can_spend_spp(&state, &user, &space_id_vo, &team).await {
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = IncreaseStatCommand {
        player_id: player.id.clone(),
        stat,
    };

    match increase_stat_use_case::execute(
        cmd,
        state.players.repository.as_ref(),
        state.players.skill_catalog.as_ref(),
        &state.players.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header("HX-Refresh", "true")
            .body(Body::empty())
            .unwrap(),
        Err(increase_stat_use_case::IncreaseStatError::PlayerNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(increase_stat_use_case::IncreaseStatError::Domain(e)) => {
            tracing::warn!("post_increase_stat domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(increase_stat_use_case::IncreaseStatError::Repository(e)) => {
            tracing::error!("post_increase_stat repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn parse_stat(raw: &str) -> Option<StatKind> {
    match raw {
        "ma" => Some(StatKind::Ma),
        "st" => Some(StatKind::St),
        "ag" => Some(StatKind::Ag),
        "pa" => Some(StatKind::Pa),
        "av" => Some(StatKind::Av),
        _ => None,
    }
}
