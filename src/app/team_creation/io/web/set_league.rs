use crate::app::auth::auth_backend::AuthSession;
use crate::app::team_creation::domain::roster::LeagueId;
use crate::app::team_creation::use_cases::build_team::set_league as uc;
use crate::app::team_creation::use_cases::build_team::set_league::SetLeagueCommand;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SetLeagueBody {
    pub league_id: String,
}

pub async fn set_league(
    _auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SetLeagueBody>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = SetLeagueCommand {
        team_id: team_entity_id,
        space_id,
        league_id: LeagueId(body.league_id.clone()),
    };

    match uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Err(uc::SetLeagueError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(uc::SetLeagueError::Repository(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(()) => axum::http::Response::builder()
            .status(200)
            .header(
                "HX-Trigger",
                json!({"leagueSelected": {"league_id": body.league_id}}).to_string(),
            )
            .body(axum::body::Body::empty())
            .unwrap()
            .into_response(),
    }
}
