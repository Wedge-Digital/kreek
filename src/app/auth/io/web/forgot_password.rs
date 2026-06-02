use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::routes::{path, Routes};
use crate::app::auth::use_cases::send_reset_password_email::{
    execute, SendResetPasswordEmailCommand, SendResetPasswordEmailError,
};
use crate::app::shared_kernel::coach_name::CoachName;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template, Default)]
#[template(path = "auth-forgot-password.html")]
pub struct AuthForgotPassword {
    pub error: Option<String>,
    pub routes: Routes,
}

impl IntoResponse for AuthForgotPassword {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template, Default)]
#[template(path = "auth-forgot-password-sent.html")]
pub struct AuthForgotPasswordSent {
    pub routes: Routes,
}

impl IntoResponse for AuthForgotPasswordSent {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn display_forgot_password(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        AuthForgotPassword::default().into_response()
    } else {
        match AuthForgotPassword::default().render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn display_forgot_password_sent(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        AuthForgotPasswordSent::default().into_response()
    } else {
        match AuthForgotPasswordSent::default().render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct LostPasswordPayload {
    pub coach_name: String,
}

pub async fn post_forgot_password(
    State(state): State<AppState>,
    Form(payload): Form<LostPasswordPayload>,
) -> impl IntoResponse {
    let coach_name = match CoachName::try_new(&payload.coach_name) {
        Ok(v) => v,
        Err(_) => {
            return AuthForgotPassword {
                error: Some("Nom de coach invalide.".into()),
                ..Default::default()
            }
            .into_response()
        }
    };

    match execute(
        SendResetPasswordEmailCommand {
            coach_name,
            host_domain: state.host_domain.clone(),
        },
        state.auth.user_repository.as_ref(),
        state.auth.reset_token_repository.as_ref(),
        state.email_service.as_ref(),
    )
    .await
    {
        Ok(()) | Err(SendResetPasswordEmailError::CoachNameNotFound) => {
            let mut response = Response::new(Body::empty());
            response.headers_mut().insert(
                header::HeaderName::from_static("hx-redirect"),
                header::HeaderValue::from_static(path::FORGOT_PASSWORD_SENT),
            );
            response
        }
        Err(_) => AuthForgotPassword {
            error: Some("Erreur interne, réessaie plus tard.".into()),
            ..Default::default()
        }
        .into_response(),
    }
}
