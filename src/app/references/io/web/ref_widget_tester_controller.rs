use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub async fn widget_tester_controller(State(_state): State<AppState>) -> impl IntoResponse {
    ReferencesWidgetPageTesterTemplate {
        routes: AppRoutes::default(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "pages/widget-tester-page.html")]
pub struct ReferencesWidgetPageTesterTemplate {
    pub routes: AppRoutes,
}

impl IntoResponse for ReferencesWidgetPageTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
