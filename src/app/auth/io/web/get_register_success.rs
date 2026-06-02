use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::routes::Routes;
use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Template, Default)]
#[template(path = "auth-register-success.html")]
pub struct RegisterSuccessTemplate {
    pub routes: Routes,
}

impl IntoResponse for RegisterSuccessTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize)]
pub struct RegisterFormPayload {
    pub coach_name: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

pub async fn register_success(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        RegisterSuccessTemplate::default().into_response()
    } else {
        match RegisterSuccessTemplate::default().render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
