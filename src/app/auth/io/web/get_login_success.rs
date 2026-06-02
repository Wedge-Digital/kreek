use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::routes::Routes;
use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template, Default)]
#[template(path = "auth-login-success.html")]
pub struct LoginSuccessTemplate {
    pub routes: Routes,
}

impl IntoResponse for LoginSuccessTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn login_success(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        LoginSuccessTemplate::default().into_response()
    } else {
        match LoginSuccessTemplate::default().render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
