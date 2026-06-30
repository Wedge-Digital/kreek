use crate::app::routes::AppRoutes;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::shared_kernel::roster_definition::RosterDefinition;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RosterPickerParams {
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub selected: String,
    #[serde(default)]
    pub select_all: bool,
}

pub async fn roster_picker_controller(
    State(state): State<AppState>,
    Query(params): Query<RosterPickerParams>,
) -> impl IntoResponse {
    let rosters = state.references.repository.list_roster_definitions();

    let selected: Vec<String> = if params.select_all {
        rosters.iter().map(|r| r.id.to_string()).collect()
    } else {
        params
            .selected
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };

    RosterPickerTemplate {
        routes: AppRoutes::default(),
        rosters,
        instance_id: params.instance_id,
        selected_json: serde_json::to_string(&selected).unwrap_or_else(|_| "[]".to_string()),
    }.into_response()
}

#[derive(Template)]
#[template(path = "widgets/roster-picker.html")]
pub struct RosterPickerTemplate {
    pub routes: AppRoutes,
    pub rosters: Vec<RosterDefinition>,
    pub instance_id: String,
    pub selected_json: String,
}

impl IntoResponse for RosterPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}