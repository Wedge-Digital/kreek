use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::routes::path as auth_path;
use crate::app::shared_kernel::common_types::CloudinaryImage;
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::routes::path;
use crate::app::spaces::uses_cases::register_new_space::{
    execute, RegisterNewSpaceCommand, RegisterSpaceError,
};
use crate::app::routes::AppRoutes;
use crate::app::spaces::context::SpacesContext;
use askama::Template;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template, Default)]
#[template(path = "new-space.html")]
pub struct NewSpaceTemplate {
    pub app_routes: AppRoutes,
    pub space_name_value: String,
    pub space_name_error: Option<String>,
    pub logo_url_value: String,
    pub logo_error: Option<String>,
}

impl IntoResponse for NewSpaceTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template, Default)]
#[template(path = "new-space-form.html")]
pub struct NewSpaceFormTemplate {
    pub app_routes: AppRoutes,
    pub space_name_value: String,
    pub space_name_error: Option<String>,
    pub logo_url_value: String,
    pub logo_error: Option<String>,
}

impl IntoResponse for NewSpaceFormTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn register_space() -> impl IntoResponse {
    NewSpaceTemplate::default().into_response()
}

#[derive(Deserialize)]
pub struct RegisterSpaceFormPayload {
    pub space_name: String,
    pub logo_url: String,
}

pub async fn register_space_submit(
    auth_session: AuthSession,
    State(ctx): State<SpacesContext>,
    Form(payload): Form<RegisterSpaceFormPayload>,
) -> impl IntoResponse {
    let mut form = NewSpaceFormTemplate {
        space_name_value: payload.space_name.clone(),
        logo_url_value: payload.logo_url.clone(),
        ..Default::default()
    };

    // Validation du nom d'espace
    let space_name = match SpaceName::try_new(&payload.space_name) {
        Ok(v) => Some(v),
        Err(_) => {
            form.space_name_error = Some(
                "Le nom ne peut contenir que des lettres, chiffres, tirets et underscores (100 caractères max).".into(),
            );
            None
        }
    };

    // Validation du logo
    let space_logo = match CloudinaryImage::try_new(payload.logo_url.clone()) {
        Ok(v) => Some(v),
        Err(_) => {
            form.logo_error = Some("Veuillez uploader un logo pour votre espace.".into());
            None
        }
    };

    let (Some(space_name), Some(space_logo)) = (space_name, space_logo) else {
        return form.into_response();
    };

    let Some(user) = auth_session.user else {
        return Response::builder()
            .header("HX-Redirect", auth_path::AUTH_LAYOUT)
            .body(Body::empty())
            .unwrap();
    };
    let coach_id = user.id;

    let cmd = RegisterNewSpaceCommand {
        coach_id,
        space_name,
        space_logo,
    };

    match execute(
        cmd,
        ctx.space_repository.as_ref(),
        ctx.user_cache_repository.as_ref(),
        &ctx.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header("HX-Redirect", path::NEW_SPACE)
            .body(Body::empty())
            .unwrap(),

        Err(RegisterSpaceError::SpaceNameAlreadyTaken) => {
            form.space_name_error = Some("Ce nom d'espace est déjà utilisé.".into());
            form.into_response()
        }

        Err(RegisterSpaceError::CoachNotFound) => {
            form.space_name_error =
                Some("Votre profil est introuvable, veuillez vous reconnecter.".into());
            form.into_response()
        }

        Err(RegisterSpaceError::Database(_)) => {
            form.space_name_error = Some("Erreur interne, veuillez réessayer.".into());
            form.into_response()
        }
    }
}
