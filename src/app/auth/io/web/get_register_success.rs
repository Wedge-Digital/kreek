use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::auth::routes::Routes;

#[derive(Template, Default)]
#[template(path = "auth-register-success.html")]
pub struct RegisterSuccessTemplate {
    pub routes: Routes,
}

impl IntoResponse for RegisterSuccessTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize)]
pub struct RegisterFormPayload {
    pub coach_name:       String,
    pub email:            String,
    pub password:         String,
    pub password_confirm: String,
}

pub async fn register_success() -> impl IntoResponse {
    RegisterSuccessTemplate::default()
}
