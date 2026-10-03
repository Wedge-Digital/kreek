//! Les décisions d'inscription — approuver, rejeter, renvoyer, tout approuver.
//!
//! **Gardées depuis la carte 573.** Elles ne l'étaient par rien : leurs boutons
//! ne s'affichaient qu'aux admins, mais n'importe quel connecté pouvait
//! décider des inscriptions d'une compétition par une requête forgée. Elles
//! exigent désormais un admin de l'espace ou de la compétition de l'équipe —
//! `garde_commissaire`, la règle de tout kreek.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::teams::io::web::garde_commissaire::exiger_commissaire;
use crate::app::teams::use_cases::approve_enrollment::{self, ApproveEnrollmentError};
use crate::app::teams::use_cases::commands::RejectEnrollmentCommand;
use crate::app::teams::use_cases::reject_enrollment::{self, RejectEnrollmentError};
use crate::app::teams::use_cases::roster_edit_access_service::est_admin_de_l_equipe;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

fn enrollment_changed() -> Response {
    Response::builder()
        .header("HX-Trigger", "enrollmentChanged")
        .body(Body::empty())
        .unwrap()
}

fn error_response(status: StatusCode) -> Response {
    status.into_response()
}

pub async fn approve_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    if let Err(refus) = exiger_commissaire(&state, &auth_session, &team_id).await {
        return refus;
    }
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    match approve_enrollment::execute(&team_entity_id, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(ApproveEnrollmentError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(ApproveEnrollmentError::Domain(_)) => error_response(StatusCode::UNPROCESSABLE_ENTITY),
        Err(ApproveEnrollmentError::Repository(e)) => {
            tracing::error!("approve_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn reject_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    if let Err(refus) = exiger_commissaire(&state, &auth_session, &team_id).await {
        return refus;
    }
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    let cmd = RejectEnrollmentCommand {
        team_id: team_entity_id,
    };

    match reject_enrollment::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(RejectEnrollmentError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(RejectEnrollmentError::Domain(_)) => error_response(StatusCode::UNPROCESSABLE_ENTITY),
        Err(RejectEnrollmentError::Repository(e)) => {
            tracing::error!("reject_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn dismiss_enrollment(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    if let Err(refus) = exiger_commissaire(&state, &auth_session, &team_id).await {
        return refus;
    }
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST),
    };

    use crate::app::teams::use_cases::commands::DismissTeamCommand;
    use crate::app::teams::use_cases::dismiss_team;

    let cmd = DismissTeamCommand {
        team_id: team_entity_id.clone(),
        space_id: EntityId::new(),
        admin_id: EntityId::new(),
    };

    match dismiss_team::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => enrollment_changed(),
        Err(dismiss_team::DismissTeamError::TeamNotFound) => error_response(StatusCode::NOT_FOUND),
        Err(dismiss_team::DismissTeamError::Domain(_)) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY)
        }
        Err(dismiss_team::DismissTeamError::Repository(e)) => {
            tracing::error!("dismiss_enrollment: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct ApproveAllParams {
    pub competition_id: String,
    pub season_id: String,
}

/// **Le droit se vérifie équipe par équipe**, sur l'espace et la compétition
/// de chacune — pas sur la `competition_id` de la requête : `teams` ne sait pas
/// vérifier que la saison demandée lui appartient, et une URL forgée pourrait
/// associer une compétition qu'on administre à la saison d'une autre.
///
/// Un visiteur qui n'administre aucune des équipes en attente reçoit `403` ;
/// une équipe qu'il n'administre pas, au milieu d'autres, est laissée en
/// attente.
pub async fn approve_all_enrollments(
    Query(params): Query<ApproveAllParams>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let Some(user) = auth_session.user.clone() else {
        return error_response(StatusCode::UNAUTHORIZED);
    };
    let pending = match state
        .teams
        .team_repository
        .find_by_season_and_status(&params.season_id, "PendingEnrollment")
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("approve_all_enrollments: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut autorisees = 0usize;
    for row in &pending {
        if !administre(&state, &user.id, &row.team_id).await {
            continue;
        }
        autorisees += 1;
        let team_id = match EntityId::try_new(&row.team_id) {
            Ok(id) => id,
            Err(_) => continue,
        };
        if let Err(e) =
            approve_enrollment::execute(&team_id, state.teams.team_repository.as_ref()).await
        {
            tracing::warn!("approve_all skip {}: {e:?}", row.team_id);
        }
    }

    if !pending.is_empty() && autorisees == 0 {
        return error_response(StatusCode::FORBIDDEN);
    }
    enrollment_changed()
}

/// Admin de l'espace ou de la compétition de cette équipe. Une équipe
/// introuvable ou illisible n'est administrée par personne.
async fn administre(
    state: &AppState,
    user_id: &crate::app::shared_kernel::identity::ids::CoachId,
    team_id: &str,
) -> bool {
    match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(team)) => {
            est_admin_de_l_equipe(&team, user_id, state.teams.admin_access.as_ref()).await
        }
        _ => false,
    }
}
