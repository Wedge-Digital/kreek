use crate::app::routes::AppRoutes;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::shared_kernel::inducement_definition::InducementDefinition;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct InducementPickerParams {
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub selected: String,
    #[serde(default)]
    pub select_all: bool,
}

pub async fn inducement_picker_controller(
    State(state): State<AppState>,
    Query(params): Query<InducementPickerParams>,
) -> impl IntoResponse {
    let inducements = state.references.repository.list_inducements();

    let selected: Vec<String> = if params.select_all {
        inducements.iter().map(|i| i.id.to_string()).collect()
    } else {
        params
            .selected
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };

    InducementPickerTemplate {
        routes: AppRoutes::default(),
        inducements,
        instance_id: params.instance_id,
        selected_json: serde_json::to_string(&selected).unwrap_or_else(|_| "[]".to_string()),
    }.into_response()
}

#[derive(Template)]
#[template(path = "widgets/inducement-picker.html")]
pub struct InducementPickerTemplate {
    pub routes: AppRoutes,
    pub inducements: Vec<InducementDefinition>,
    pub instance_id: String,
    pub selected_json: String,
}

impl IntoResponse for InducementPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}