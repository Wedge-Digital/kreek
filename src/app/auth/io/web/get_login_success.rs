use crate::app::auth::context::AuthContext;
use crate::app::auth::io::web::auth_layout::AuthLayout;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template, Default)]
#[template(path = "auth-login-success.html")]
pub struct LoginSuccessTemplate {
    pub authenticated_home: String,
}

impl IntoResponse for LoginSuccessTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn login_success(
    State(ctx): State<AuthContext>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let page = || LoginSuccessTemplate {
        authenticated_home: ctx.authenticated_home.clone(),
    };

    if headers.contains_key("hx-request") {
        page().into_response()
    } else {
        match page().render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
