use askama::Template;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::routes::path;
use crate::app::spaces::uses_cases::register_new_space::{execute, RegisterNewSpaceCommand, RegisterSpaceError};
use crate::app::team_creation::routes::Routes;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template, Default)]
#[template(path = "new-space.html")]
pub struct NewSpaceTemplate {
    pub routes:            Routes,
    pub space_name_value:  String,
    pub space_name_error:  Option<String>,
    pub logo_url_value:    String,
    pub logo_error:        Option<String>,
}

impl IntoResponse for NewSpaceTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn register_space(headers: axum::http::HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        NewSpaceTemplate::default().into_response()
    } else {
        let content = NewSpaceTemplate::default().render().unwrap_or_default();
        AppLayout { content, routes: WebRoutes }.into_response()
    }
}

#[derive(Deserialize)]
pub struct RegisterSpaceFormPayload {
    pub space_name: String,
    pub logo_url:   String,
}

pub async fn register_space_submit(
    State(state): State<AppState>,
    Form(payload): Form<RegisterSpaceFormPayload>,
) -> impl IntoResponse {
    let mut tmpl = NewSpaceTemplate {
        space_name_value: payload.space_name.clone(),
        logo_url_value:   payload.logo_url.clone(),
        ..Default::default()
    };

    // Validation du nom d'espace
    let space_name = match SpaceName::try_new(&payload.space_name) {
        Ok(v)  => Some(v),
        Err(_) => {
            tmpl.space_name_error = Some(
                "Le nom ne peut contenir que des lettres, chiffres, tirets et underscores (100 caractères max).".into(),
            );
            None
        }
    };

    // Validation du logo
    let space_logo = match CloudinaryImage::try_new(payload.logo_url.clone()) {
        Ok(v)  => Some(v),
        Err(_) => {
            tmpl.logo_error = Some("Veuillez uploader un logo pour votre espace.".into());
            None
        }
    };

    let (Some(space_name), Some(space_logo)) = (space_name, space_logo) else {
        return tmpl.into_response();
    };

    // TODO: récupérer le CoachId depuis AuthSession (axum-login) — pas encore câblé.
    // À remplacer par : auth_session.user.map(|u| u.id).ok_or(401) une fois le middleware en place.
    let coach_id = CoachId::new();

    let cmd = RegisterNewSpaceCommand { coach_id, space_name, space_logo };

    match execute(cmd, state.space_repository.as_ref()).await {
        Ok(()) => Response::builder()
            .header("HX-Redirect", path::NEW_SPACE)
            .body(Body::empty())
            .unwrap(),

        Err(RegisterSpaceError::SpaceNameAlreadyTaken) => {
            tmpl.space_name_error = Some("Ce nom d'espace est déjà utilisé.".into());
            tmpl.into_response()
        }

        Err(RegisterSpaceError::Database(_)) => {
            tmpl.space_name_error = Some("Erreur interne, veuillez réessayer.".into());
            tmpl.into_response()
        }
    }
}