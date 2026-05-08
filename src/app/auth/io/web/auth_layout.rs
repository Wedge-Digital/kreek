use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use crate::app::auth::routes::path;

#[derive(Template)]
#[template(path = "auth-layout.html")]
pub struct AuthLayout {
    pub content: String
}

impl IntoResponse for AuthLayout {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn auth_layout() -> impl IntoResponse {
    Redirect::to(path::LOGIN)
}
