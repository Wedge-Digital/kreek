use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::shared_kernel::space_definition::SpaceDefinition;
use crate::app::shared_kernel::space_name::SpaceName;
use crate::state::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "competitions-widget-tester-page.html")]
pub struct CompetitionsWidgetTesterTemplate {
    pub routes: AppRoutes,
    pub spaces: Vec<SpaceDefinition>,
}

impl IntoResponse for CompetitionsWidgetTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_competitions_widget_tester(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let spaces = state
        .spaces
        .space_repository
        .find_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SpaceDefinition {
            id: SpaceId::try_new(&s.id).expect(""),
            name: SpaceName::try_new(&s.name).expect(""),
        })
        .collect();

    CompetitionsWidgetTesterTemplate {
        routes: AppRoutes::default(),
        spaces,
    }
    .into_response()
}
