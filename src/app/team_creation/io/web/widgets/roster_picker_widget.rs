use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::io::web::builders::build_roster_items_with_tiers;
use crate::app::team_creation::io::web::view_models::RosterPickerItemWithTier;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "widgets/roster-picker-widget.html")]
pub struct RosterPickerWidgetTemplate {
    pub rosters: Vec<RosterPickerItemWithTier>,
    pub selected: Option<String>,
}

impl IntoResponse for RosterPickerWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn roster_picker_widget(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("roster_picker_widget draft find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_data = state.team_creation.reference_data.as_ref();
    let rosters = build_roster_items_with_tiers(ref_data, draft.creation_rules());

    let selected = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(team)) => Some(team.roster.id.0.clone()),
        _ => None,
    };

    RosterPickerWidgetTemplate { rosters, selected }.into_response()
}
