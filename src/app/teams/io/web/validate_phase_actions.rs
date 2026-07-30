use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::teams::io::web::{dismissals, recruitment};
use crate::app::teams::use_cases::commands::{
    ValidateDismissalsPhaseCommand, ValidateImprovementPhaseCommand,
    ValidateRecruitmentPhaseCommand,
};
use crate::app::teams::use_cases::{
    validate_dismissals_phase_use_case, validate_improvement_phase_use_case,
    validate_recruitment_phase_use_case,
};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Recrutement et renvois se valident depuis leur propre page, qui n'a plus
/// lieu d'être une fois la phase close : on ramène le coach à sa feuille
/// d'équipe. La validation des évolutions, elle, part de cette feuille, où un
/// simple rafraîchissement suffit.
fn redirect_response(url: String) -> Response {
    Response::builder()
        .header("HX-Redirect", url)
        .body(Body::empty())
        .unwrap()
}

fn refresh_response() -> Response {
    Response::builder()
        .header("HX-Refresh", "true")
        .body(Body::empty())
        .unwrap()
}

pub async fn post_validate_improvement_phase(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(team_id) = EntityId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = ValidateImprovementPhaseCommand { team_id };

    match validate_improvement_phase_use_case::execute(cmd, state.teams.team_repository.as_ref())
        .await
    {
        Ok(()) => refresh_response(),
        Err(validate_improvement_phase_use_case::ValidateImprovementPhaseError::TeamNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(validate_improvement_phase_use_case::ValidateImprovementPhaseError::Domain(e)) => {
            tracing::warn!("validate_improvement_phase domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(validate_improvement_phase_use_case::ValidateImprovementPhaseError::Repository(e)) => {
            tracing::error!("validate_improvement_phase repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn post_validate_recruitment_phase(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(team_id) = EntityId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = ValidateRecruitmentPhaseCommand { team_id };

    let resultat = validate_recruitment_phase_use_case::execute(
        cmd,
        state.teams.team_repository.as_ref(),
        state.teams.basket_repository.as_ref(),
        state.teams.roster_catalog_port.as_ref(),
        state.teams.squad_port.as_ref(),
    )
    .await;

    use validate_recruitment_phase_use_case::ValidateRecruitmentPhaseError as E;
    match resultat {
        Ok(()) => redirect_response(
            AppRoutes::default()
                .teams
                .team_detail(&_space_id, &team_id.to_string()),
        ),
        Err(E::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        // Le panier ne passe plus contre l'état du jour : rien n'est appliqué
        // et les lignes fautives sont nommées dans le panier.
        Err(E::BasketNoLongerValid(lignes)) => {
            recruitment::refus_en_bloc(&state, &team_id.to_string(), lignes).await
        }
        Err(E::Domain(e)) => {
            tracing::warn!("validate_recruitment_phase domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(E::Hydration(e)) => {
            tracing::error!("validate_recruitment_phase hydratation: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(E::Repository(e)) => {
            tracing::error!("validate_recruitment_phase repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn post_validate_dismissals_phase(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(team_id) = EntityId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = ValidateDismissalsPhaseCommand { team_id };

    let resultat = validate_dismissals_phase_use_case::execute(
        cmd,
        state.teams.team_repository.as_ref(),
        state.teams.basket_repository.as_ref(),
        state.teams.roster_catalog_port.as_ref(),
        state.teams.squad_port.as_ref(),
    )
    .await;

    use validate_dismissals_phase_use_case::ValidateDismissalsPhaseError as E;
    match resultat {
        // Depuis la carte 269, la validation part de la page de renvois — qui
        // n'a plus lieu d'être une fois la phase close. Un rafraîchissement y
        // rechargerait un écran dont la phase vient de changer, donc vide : on
        // ramène le coach à sa feuille d'équipe.
        Ok(()) => redirect_response(
            AppRoutes::default()
                .teams
                .team_detail(&_space_id, &team_id.to_string()),
        ),
        Err(E::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        // Le panier ne passe plus contre l'effectif du jour : rien n'est appliqué
        // et les lignes fautives sont nommées dans le panier.
        Err(E::BasketNoLongerValid(lignes)) => {
            dismissals::refus_en_bloc(&state, &team_id.to_string(), lignes).await
        }
        Err(E::Domain(e)) => {
            tracing::warn!("validate_dismissals_phase domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(E::Hydration(e)) => {
            tracing::error!("validate_dismissals_phase hydratation: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(E::Repository(e)) => {
            tracing::error!("validate_dismissals_phase repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
