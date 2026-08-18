use crate::app::auth::context::AuthContext;
use crate::app::auth::io::web::auth_layout::AuthLayout;
use crate::app::auth::routes::{path, Routes};
use crate::app::auth::use_cases::reset_password::{
    execute, ResetPasswordCommand, ResetPasswordError,
};
use crate::app::shared_kernel::identity::secret::Secret;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template, Default)]
#[template(path = "auth-update-password.html")]
pub struct UpdatePasswordTemplate {
    pub reset_token: String,
    pub token_error: Option<String>,
    pub password_error: Option<String>,
    pub password_confirm_error: Option<String>,
    pub routes: Routes,
}

impl IntoResponse for UpdatePasswordTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn display_reset_password(
    Path(reset_token): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let template = UpdatePasswordTemplate {
        reset_token,
        ..Default::default()
    };
    if headers.contains_key("hx-request") {
        template.into_response()
    } else {
        match template.render() {
            Ok(content) => AuthLayout { content }.into_response(),
            Err(e) => {
                tracing::error!("render failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct UpdatePasswordPayload {
    pub password: String,
    pub password_confirm: String,
}

pub async fn post_reset_password(
    State(ctx): State<AuthContext>,
    Path(reset_token): Path<String>,
    Form(payload): Form<UpdatePasswordPayload>,
) -> impl IntoResponse {
    let cmd = ResetPasswordCommand {
        token: Secret::new(reset_token.clone()),
        password: Secret::new(payload.password),
        password_confirm: Secret::new(payload.password_confirm),
    };

    match execute(
        cmd,
        ctx.user_repository.as_ref(),
        ctx.reset_token_repository.as_ref(),
    )
    .await
    {
        Ok(()) => {
            let mut response = Response::new(Body::empty());
            response.headers_mut().insert(
                header::HeaderName::from_static("hx-redirect"),
                header::HeaderValue::from_static(path::LOGIN),
            );
            response
        }
        Err(ResetPasswordError::TokenNotFound | ResetPasswordError::TokenExpired) => {
            UpdatePasswordTemplate {
                reset_token,
                token_error: Some(
                    "Ce lien est invalide ou a expiré. Veuillez en demander un nouveau.".into(),
                ),
                ..Default::default()
            }
            .into_response()
        }
        Err(ResetPasswordError::PasswordTooShort) => UpdatePasswordTemplate {
            reset_token,
            password_error: Some("Le mot de passe doit contenir au moins 8 caractères.".into()),
            ..Default::default()
        }
        .into_response(),
        Err(ResetPasswordError::PasswordMismatch) => UpdatePasswordTemplate {
            reset_token,
            password_confirm_error: Some("Les mots de passe ne correspondent pas.".into()),
            ..Default::default()
        }
        .into_response(),
        Err(_) => UpdatePasswordTemplate {
            reset_token,
            token_error: Some("Erreur interne, réessaie plus tard.".into()),
            ..Default::default()
        }
        .into_response(),
    }
}
