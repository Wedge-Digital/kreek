use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template, Default)]
#[template(path = "auth/auth-login-success.html")]
pub struct LoginSuccessTemplate {
}

impl IntoResponse for LoginSuccessTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn login_success() -> impl IntoResponse {
    LoginSuccessTemplate::default()
}
