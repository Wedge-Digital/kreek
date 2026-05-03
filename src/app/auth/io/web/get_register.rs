use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::io::web::get_login::LoginTemplate;

#[derive(Template, Default)]
#[template(path = "auth/auth-register.html")]
pub struct RegisterTemplate {
    pub coach_name_value:       String,
    pub email_value:            String,
    pub coach_name_error:       Option<String>,
    pub email_error:            Option<String>,
    pub password_error:         Option<String>,
    pub password_confirm_error: Option<String>,
}

impl IntoResponse for RegisterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_register(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        RegisterTemplate::default().into_response()
    } else {
        let content = RegisterTemplate::default().render().unwrap_or_default();
        AuthLayout { content }.into_response()
    }
}
