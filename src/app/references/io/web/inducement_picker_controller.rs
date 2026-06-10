use crate::app::routes::AppRoutes;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::shared_kernel::inducement_definition::InducementDefinition;
use crate::state::AppState;

pub async fn inducement_picker_controller(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    InducementPickerTemplate {
        routes: AppRoutes::default(),
        inducements: _state.references.repository.list_inducements()
    }.into_response()
}

#[derive(Template)]
#[template(path = "widgets/inducement-picker.html")]
pub struct InducementPickerTemplate {
    pub routes: AppRoutes,
    pub inducements: Vec<InducementDefinition>
}

impl IntoResponse for InducementPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}