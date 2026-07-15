use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId};
use crate::app::players::domain::value_objects::SkillId;
use crate::app::players::use_cases::commands::PurchaseSkillCommand;
use crate::app::players::use_cases::purchase_skill_use_case;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::teams::domain::team::{GamePhase, Team};
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

    let player = match state.players.repository.find_by_id(&PlayerId(player_id.clone())).await {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("post_purchase_skill find player {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let team = match state.teams.team_repository.find_by_id(&player.team_id.0).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("post_purchase_skill find team {}: {e}", player.team_id.0);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let space_id_vo = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if team.game_phase != Some(GamePhase::PlayerImprovement) {
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

    let cmd = PurchaseSkillCommand { player_id: player.id.clone(), skill_id, mode };

    match purchase_skill_use_case::execute(cmd, state.players.repository.as_ref(), state.players.skill_catalog.as_ref()).await {
        Ok(()) => Response::builder().header("HX-Refresh", "true").body(Body::empty()).unwrap(),
        Err(purchase_skill_use_case::PurchaseSkillError::PlayerNotFound) => StatusCode::NOT_FOUND.into_response(),
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

/// Coach de l'équipe, admin de compétition, ou admin d'espace — même patron
/// que `check_admin_rights` de `player_detail_controller.rs`, étendu au coach.
pub async fn can_spend_spp(
    state: &AppState,
    user: &crate::app::shared_kernel::user::User,
    space_id: &SpaceId,
    team: &Team,
) -> bool {
    if team.coach_id.to_string() == user.id.to_string() {
        return true;
    }
    let is_space_admin = matches!(
        state.spaces.space_repository.find_member_profile(&user.id, space_id).await,
        Ok(Some(SpaceProfile::SpaceAdmin))
    );
    if is_space_admin {
        return true;
    }
    let Some(competition_id) = &team.competition_id else {
        return false;
    };
    let user_id_str = user.id.to_string();
    let coach_name_str = user.coach_name.clone().into_inner();
    match state.competitions.competition_repository.find_base_info(competition_id).await {
        Ok(Some(info)) => info.admin_ids.contains(&user_id_str) || info.admin_names.contains(&coach_name_str),
        _ => false,
    }
}

