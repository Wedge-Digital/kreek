use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::use_cases::apply_costly_mistakes_use_case::{
    self, ApplyCostlyMistakesCommand, ApplyCostlyMistakesError,
};
use crate::app::teams::use_cases::roster_edit_access_service;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Lance le dé des erreurs coûteuses et l'applique.
///
/// **Aucun extracteur de corps** : l'équipe est dans le chemin, le coach dans la
/// session. Rien n'entre, donc rien n'est à valider — et le client ne peut pas
/// proposer de jet.
pub async fn post_costly_mistakes_roll(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(team_id_vo) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    // Le droit garde le **POST**, pas seulement l'affichage : l'URL est
    // devinable, et un jet a un effet financier.
    let Ok(Some(team)) = state.teams.team_repository.find_by_id(&team_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let autorise = roster_edit_access_service::peut_modifier_effectif(
        &team,
        &user.id,
        &user.coach_name.clone().into_inner(),
        state.teams.access_port.as_ref(),
    )
    .await;
    if !autorise {
        tracing::warn!(
            team_id = %team_id,
            user_id = %user.id,
            "jet refusé : ni propriétaire de l'équipe, ni administrateur"
        );
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = ApplyCostlyMistakesCommand {
        team_id: team_id_vo,
    };
    match apply_costly_mistakes_use_case::execute(
        cmd,
        state.teams.team_repository.as_ref(),
        state.teams.dice.as_ref(),
    )
    .await
    {
        Ok(_issue) => redirection(&space_id, &team_id),
        Err(ApplyCostlyMistakesError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        // **409 et non 422** : la requête est bien formée, c'est l'état qui a
        // changé. Typiquement un second jet — `CostlyMistakesApplied` a reposé
        // `ReadyToPlay`, donc la garde de phase du domaine refuse. L'idempotence
        // ne demande ni verrou ni jeton, elle sort du modèle.
        Err(ApplyCostlyMistakesError::Domain(e)) => {
            tracing::warn!(team_id = %team_id, "jet refusé : {e:?}");
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("post_costly_mistakes_roll: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// La feuille d'équipe, qui affiche déjà la phase. L'écran du jet et son
/// résultat détaillé arrivent avec la carte 410.
fn redirection(space_id: &str, team_id: &str) -> Response {
    Response::builder()
        .header(
            "HX-Redirect",
            AppRoutes::default().teams.team_detail(space_id, team_id),
        )
        .body(Body::empty())
        .unwrap()
        .into_response()
}
