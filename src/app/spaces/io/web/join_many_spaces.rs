use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Form;
use axum::http::StatusCode;
use serde::Deserialize;
use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::spaces::domain::space_repository_port::space_repository_port::SpaceRepositoryError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct JoinSpacesForm {
    #[serde(default)]
    pub space_ids: Vec<String>,
}

pub async fn join_many_spaces(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(payload): Form<JoinSpacesForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mut first_joined: Option<String> = None;

    for raw_id in &payload.space_ids {
        let space_id = match SpaceId::try_new(raw_id) {
            Ok(id) => id,
            Err(_) => continue,
        };

        match state.spaces.space_repository.add_member(&space_id, &user.id, &SpaceProfile::SimpleUser).await {
            Ok(()) => {
                if first_joined.is_none() {
                    first_joined = Some(raw_id.clone());
                }
            }
            Err(SpaceRepositoryError::CoachAlreadyMember) => {}
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    let redirect_to = match first_joined {
        Some(id) => crate::app::news::routes::path::APP_HOME.replace("{space_id}", &id),
        None     => crate::app::spaces::routes::path::SPACE_ALL.to_string(),
    };

    Response::builder()
        .header("HX-Redirect", redirect_to)
        .body(axum::body::Body::empty())
        .unwrap()
}