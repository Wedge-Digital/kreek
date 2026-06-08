use crate::app::routes::AppRoutes;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use sqlx::query::Query;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::shared_kernel::RosterDefinition::RosterDefinition;
use crate::app::team_creation::domain::roster::Roster;
use crate::state::AppState;

pub async fn roster_picker_controller(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    RosterPickerTemplate {
        routes: AppRoutes::default(),
        rosters: _state.references.repository.list_roster_definitions()
    }.into_response()
}

#[derive(Template)]
#[template(path = "widgets/roster-picker.html")]
pub struct RosterPickerTemplate {
    pub routes: AppRoutes,
    pub rosters: Vec<RosterDefinition>
}

impl IntoResponse for RosterPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}