//! Les gestes de commissaire sur une équipe — ajuster sa trésorerie, la
//! renvoyer : admin de son espace ou de sa compétition, **jamais son seul
//! propriétaire** (carte 570).
//!
//! Ils passaient par `SpacePermissions::is_admin()`, qui ne connaît que l'admin
//! d'espace : l'admin de compétition, qui peut tout le reste sur la fiche, en
//! était exclu sans que ce fût voulu. La règle est désormais celle de tout
//! kreek, `est_admin`, appliquée à l'espace et à la compétition **de l'équipe**.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::teams::use_cases::roster_edit_access_service::est_admin_de_l_equipe;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Rend le commissaire, ou la réponse qui le refuse : 401 sans session, 404
/// pour une équipe introuvable, 403 pour tout autre visiteur — propriétaire
/// compris. Un dépôt en échec rend 500 : on ne décide pas d'un droit sur une
/// lecture ratée.
pub async fn exiger_commissaire(
    state: &AppState,
    auth_session: &AuthSession,
    team_id: &str,
) -> Result<User, Response> {
    let Some(user) = auth_session.user.clone() else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let team = match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(team)) => team,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("exiger_commissaire find_by_id {team_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    match est_admin_de_l_equipe(&team, &user.id, state.teams.admin_access.as_ref()).await {
        true => Ok(user),
        false => Err(StatusCode::FORBIDDEN.into_response()),
    }
}
