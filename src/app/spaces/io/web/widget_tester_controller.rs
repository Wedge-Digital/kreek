use crate::app::routes::AppRoutes;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::shared_kernel::space_definition::SpaceDefinition;
use crate::app::shared_kernel::space_name::SpaceName;
use crate::state::AppState;


#[derive(Template)]
#[template(path = "pages/spaces-widget-tester-page.html")]
pub struct SpacesWidgetPageTesterTemplate {
    pub routes: AppRoutes,
    pub spaces: Vec<SpaceDefinition>,
}

pub async fn space_widget_tester_controller(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let spaces = state.spaces.space_repository.find_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SpaceDefinition {
            id: SpaceId::try_new(s.id).expect(""),
            name: SpaceName::try_new(s.name).expect("") })
        .collect();

    SpacesWidgetPageTesterTemplate {
        routes: AppRoutes::default(),
        spaces,
    }.into_response()
}

impl IntoResponse for SpacesWidgetPageTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}