//! Le contrôle d'accès de la customisation, partagé par le panneau et les sept
//! mutations.
//!
//! Il vit à part parce que ses deux appelants **n'en tirent pas la même
//! conséquence** : le panneau retombe sur le journal, les endpoints répondent
//! `403`. Seule la question est commune ; la réponse ne l'est pas.
//!
//! Masquer un bouton n'est pas un contrôle d'accès — d'où la vérification sur
//! chacun des sept endpoints, et pas seulement à l'ouverture du panneau.

use crate::app::auth::domain::user::User;
use crate::app::players::domain::player::PlayerId;
use crate::app::players::io::web::player_detail_controller::can_customise;
use crate::app::players::ports::TeamRosterInfoDto;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// `Err` porte la réponse d'échec — joueur ou équipe introuvable, espace mal
/// formé. `Ok(false)` est un refus de droit, que l'appelant traduit à sa façon.
///
/// Les deux ne se confondent pas : le premier est une erreur, le second une
/// décision.
pub async fn autoriser(
    state: &AppState,
    user: &User,
    space_id: &str,
    player_id: &str,
) -> Result<bool, Response> {
    let Ok(space_id_vo) = SpaceId::try_new(space_id) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    let team = charger_equipe(state, player_id).await?;
    let coach_name = user.coach_name.clone().into_inner();
    Ok(can_customise(state, &user.id, &coach_name, &space_id_vo, &team).await)
}

/// L'équipe du joueur — c'est elle qui porte la compétition, donc l'admin
/// susceptible d'autoriser la customisation.
async fn charger_equipe(state: &AppState, player_id: &str) -> Result<TeamRosterInfoDto, Response> {
    let player = match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.to_string()))
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("customisation_access find_by_id {player_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    state
        .players
        .roster_port
        .find_team_info(&player.team_id.0)
        .await
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())
}

/// Garde des endpoints : session absente → `401`, droit refusé → `403`.
///
/// Le panneau, lui, n'utilise pas cette fonction : un `403` y remplacerait le
/// contenu du slot par une page d'erreur, là où le repli sur le journal est le
/// comportement attendu.
pub async fn garde(
    state: &AppState,
    user: Option<&User>,
    space_id: &str,
    player_id: &str,
) -> Result<(), Response> {
    let Some(user) = user else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    match autoriser(state, user, space_id, player_id).await? {
        true => Ok(()),
        false => Err(StatusCode::FORBIDDEN.into_response()),
    }
}
