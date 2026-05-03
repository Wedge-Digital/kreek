use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::io::web::get_login::LoginTemplate;

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

pub async fn login_success(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        LoginSuccessTemplate::default().into_response()
    } else {
        let content = LoginSuccessTemplate::default().render().unwrap_or_default();
        AuthLayout { content }.into_response()
    }
}
