use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
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

pub async fn get_competitions_widget_tester(State(state): State<AppState>) -> impl IntoResponse {
    // Via le port ACL, et non `state.spaces` : `competitions` n'a pas à lire le
    // repository d'un autre BC, fût-ce depuis une page de test (carte 296).
    let spaces = state.competitions.space_member_port.find_all_spaces().await;

    CompetitionsWidgetTesterTemplate {
        routes: AppRoutes::default(),
        spaces,
    }
    .into_response()
}
