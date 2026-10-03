use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::player::AcquisitionMode;
use crate::app::players::domain::value_objects::SkillId;
use crate::app::players::io::web::player_loader::charger_joueur;
use crate::app::players::ports::TeamRosterInfoDto;
use crate::app::players::use_cases::commands::PurchaseSkillCommand;
use crate::app::players::use_cases::{player_access_service, purchase_skill_use_case};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PurchaseSkillForm {
    pub skill_id: String,
    pub mode: String,
}

pub async fn post_purchase_skill(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    axum::Json(form): axum::Json<PurchaseSkillForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let player = match charger_joueur(&state, &player_id).await {
        Ok(p) => p,
        Err(refus) => return refus,
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

    let Ok(skill_id) = SkillId::try_new(form.skill_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let mode = match form.mode.as_str() {
        "chosen" => AcquisitionMode::Chosen,
        "random" => AcquisitionMode::Random,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = PurchaseSkillCommand {
        player_id: player.id.clone(),
        skill_id,
        mode,
    };

    match purchase_skill_use_case::execute(
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
        Err(purchase_skill_use_case::PurchaseSkillError::PlayerNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(purchase_skill_use_case::PurchaseSkillError::Cost(e)) => {
            tracing::warn!("post_purchase_skill cost resolution: {e:?}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(purchase_skill_use_case::PurchaseSkillError::Domain(e)) => {
            tracing::warn!("post_purchase_skill domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(purchase_skill_use_case::PurchaseSkillError::Repository(e)) => {
            tracing::error!("post_purchase_skill repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Coach de l'équipe, admin de son espace ou de sa compétition — la règle vit
/// dans `player_access_service` (carte 572).
pub async fn can_spend_spp(
    state: &AppState,
    user: &crate::app::auth::domain::user::User,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    player_access_service::peut_depenser_des_spp(
        state.players.admin_access.as_ref(),
        &user.id,
        space_id,
        team,
    )
    .await
}
