use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::use_cases::join_spaces::{execute, JoinSpacesCommand};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct JoinSpacesForm {
    #[serde(default)]
    pub space_ids: Vec<String>,
}

pub async fn join_spaces(
    auth_session: AuthSession,
    State(ctx): State<SpacesContext>,
    Json(payload): Json<JoinSpacesForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let space_ids: Vec<SpaceId> = payload
        .space_ids
        .iter()
        .filter_map(|raw| SpaceId::try_new(raw).ok())
        .collect();

    let first_id = space_ids.first().map(|id| id.to_string());

    let cmd = JoinSpacesCommand {
        coach_id: user.id,
        space_ids,
    };

    if execute(cmd, &*ctx.space_repository, &ctx.event_bus)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // L'accueil d'un espace appartient à l'hôte : il fournit la destination,
    // le BC ne connaît pas la forme de cette route.
    let redirect_to = match first_id {
        Some(id) => ctx.host_layout.space_home(&id),
        None => crate::app::spaces::routes::path::SPACE_ALL.to_string(),
    };

    Response::builder()
        .header("HX-Redirect", redirect_to)
        .body(axum::body::Body::empty())
        .unwrap()
}
