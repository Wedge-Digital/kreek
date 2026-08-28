//! Les trois mutations de la répartition en poules.
//!
//! **Chacune commence par `require_admin_access`** (carte 416) : jusqu'à elle,
//! aucune des treize routes de mutation de l'administration n'acceptait
//! `AuthSession`, et n'importe quel membre connecté pouvait réinitialiser les
//! poules d'une compétition qu'il ne gère pas.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::admin_scope::{
    equipe_de_la_saison, groupe_de_la_saison,
};
use crate::app::competitions::use_cases::admin::{assign_team_to_group, random_draw, reset_groups};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn groups_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "groupsChanged")
        .body(Body::empty())
        .unwrap()
}

pub async fn post_random_draw(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    match random_draw::execute(
        &season_id,
        state.competitions.group_repository.as_ref(),
        state.competitions.team_info_port.as_ref(),
    )
    .await
    {
        Ok(()) => groups_changed(),
        Err(random_draw::DrawError::NoGroups) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(random_draw::DrawError::NoTeams) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(e) => {
            tracing::error!("random_draw: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn post_reset_groups(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    match reset_groups::execute(&season_id, state.competitions.group_repository.as_ref()).await {
        Ok(()) => groups_changed(),
        Err(e) => {
            tracing::error!("reset_groups: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct AssignBody {
    pub team_id: String,
    pub group_id: String,
}

/// **Deux cibles, toutes deux hors du chemin.** Le groupe et l'équipe viennent
/// du corps ; ni l'un ni l'autre n'est vu par `space_scope`, qui ne résout que
/// les paramètres de chemin. Sans les deux contrôles ci-dessous, un
/// administrateur légitime pourrait affecter n'importe quelle équipe dans
/// n'importe quel groupe de la base.
pub async fn post_assign_team(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<AssignBody>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    if let Err(refus) = groupe_de_la_saison(&body.group_id, &season_id, &state).await {
        return refus;
    }
    if let Err(refus) = equipe_de_la_saison(&body.team_id, &season_id, &state).await {
        return refus;
    }
    match assign_team_to_group::execute(
        &body.team_id,
        &body.group_id,
        state.competitions.group_repository.as_ref(),
    )
    .await
    {
        Ok(()) => groups_changed(),
        Err(e) => {
            tracing::error!("assign_team: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
