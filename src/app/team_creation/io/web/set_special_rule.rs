use crate::app::auth::auth_backend::AuthSession;
use crate::app::team_creation::domain::roster::SpecialRuleId;
use crate::app::team_creation::use_cases::build_team::set_special_rule as uc;
use crate::app::team_creation::use_cases::build_team::set_special_rule::SetSpecialRuleCommand;
use crate::app::shared_kernel::common_types::EntityId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SetSpecialRuleBody {
    pub special_rule_id: String,
}

pub async fn set_special_rule(
    _auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SetSpecialRuleBody>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = SetSpecialRuleCommand {
        team_id: team_entity_id,
        space_id,
        special_rule_id: SpecialRuleId(body.special_rule_id.clone()),
    };

    match uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Err(uc::SetSpecialRuleError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(uc::SetSpecialRuleError::Repository(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(()) => axum::http::Response::builder()
            .status(200)
            .header(
                "HX-Trigger",
                json!({"specialRuleSelected": {"special_rule_id": body.special_rule_id}})
                    .to_string(),
            )
            .body(axum::body::Body::empty())
            .unwrap()
            .into_response(),
    }
}
