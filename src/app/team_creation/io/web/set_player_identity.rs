use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::domain::roster::{JerseyNumber, PlayerId};
use crate::app::team_creation::use_cases::set_player_identity as uc;
use crate::app::team_creation::use_cases::set_player_identity::SetPlayerIdentityCommand;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SetPlayerIdentityBody {
    pub name: Option<String>,
    pub jersey: u8,
}

pub async fn set_player_identity(
    _auth_session: AuthSession,
    Path((space_id, team_id, instance_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SetPlayerIdentityBody>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let jersey = match JerseyNumber::try_new(body.jersey) {
        Ok(j) => j,
        Err(_) => {
            return axum::http::Response::builder()
                .status(422)
                .header("Content-Type", "text/html")
                .body(axum::body::Body::from(
                    r#"<div class="identity-error">Le numéro de maillot doit être compris entre 1 et 99.</div>"#,
                ))
                .unwrap()
                .into_response()
        }
    };

    let cmd = SetPlayerIdentityCommand {
        team_id: team_entity_id,
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        name: body.name.unwrap_or_default().trim().to_string(),
        jersey,
    };

    match uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(uc::SetPlayerIdentityError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(uc::SetPlayerIdentityError::Domain(e)) => axum::http::Response::builder()
            .status(422)
            .header("Content-Type", "text/html")
            .body(axum::body::Body::from(format!(
                r#"<div class="identity-error">{e}</div>"#
            )))
            .unwrap()
            .into_response(),
        Err(uc::SetPlayerIdentityError::Repository(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
