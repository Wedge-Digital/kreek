use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "auth/auth-layout.html")]
pub struct AuthLayoutTemplate {
    pub content: String,
}

impl IntoResponse for AuthLayoutTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn auth_layout() -> impl IntoResponse {
    AuthLayoutTemplate { content: String::new() }
}
