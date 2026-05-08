use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::routes::Routes;

#[derive(Template, Default)]
#[template(path = "auth-login.html")]
pub struct LoginTemplate {
    pub username_value: String,
    pub password_value: String,
    pub login_error:    Option<String>,
    pub routes:         Routes,
}

impl IntoResponse for LoginTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}


pub async fn login_form(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        LoginTemplate::default().into_response()
    } else {
        let content = LoginTemplate::default().render().unwrap_or_default();
        AuthLayout { content }.into_response()
    }
}