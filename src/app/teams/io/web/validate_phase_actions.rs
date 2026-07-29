use crate::app::shared_kernel::identity::ids::EntityId;
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
        Ok(()) => refresh_response(),
        Err(E::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        // Le panier ne passe plus contre l'état du jour. La carte 264 rendra
        // les lignes fautives ; ici on refuse, sans rien appliquer.
        Err(E::BasketNoLongerValid(lignes)) => {
            tracing::warn!("panier de recrutement invalide : {lignes:?}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
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

    match validate_dismissals_phase_use_case::execute(cmd, state.teams.team_repository.as_ref())
        .await
    {
        Ok(()) => refresh_response(),
        Err(validate_dismissals_phase_use_case::ValidateDismissalsPhaseError::TeamNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(validate_dismissals_phase_use_case::ValidateDismissalsPhaseError::Domain(e)) => {
            tracing::warn!("validate_dismissals_phase domaine: {e}");
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        Err(validate_dismissals_phase_use_case::ValidateDismissalsPhaseError::Repository(e)) => {
            tracing::error!("validate_dismissals_phase repo: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
