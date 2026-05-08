use askama::Template;
use axum::extract::State;
use axum::Form;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use tracing::debug;
use crate::app::auth::io::web::get_login::LoginTemplate;
use crate::app::auth::routes::path;
use crate::app::auth::use_cases::perform_login;
use crate::app::auth::use_cases::perform_login::{LoginError, PerformLoginCommand};
use crate::app::team_creation::routes::Routes;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template, Default)]
#[template(path = "new-space.html")]
pub struct NewSpaceTemplate {
    pub routes: Routes,
}

impl IntoResponse for NewSpaceTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn register_space(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        NewSpaceTemplate::default().into_response()
    } else {
        let content = NewSpaceTemplate::default().render().unwrap_or_default();
        AppLayout { content, routes:WebRoutes }.into_response()
    }
}

pub async fn register_space_submit(
    State(state): State<AppState>,
    Form(payload): Form<PerformLoginCommand>) ->impl IntoResponse {
    debug!(coach_name = %payload.coach_name, "login form received");
}