use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::spaces::domain::space_repository_port::space_repository_port::SpaceSummary;
use crate::state::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "app-spaces.html")]
pub struct AppSpaces {
    pub app_routes: AppRoutes,
    pub spaces: Vec<SpaceSummary>,
    pub active_space_id: Option<String>,
}

impl IntoResponse for AppSpaces {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn extract_space_id(current_url: &str) -> Option<String> {
    // HX-Current-URL is a full URL: http://host/app/{space_id}/home
    // Strip scheme://host to get the path
    let path = match current_url.find("://") {
        Some(i) => current_url[i + 3..]
            .find('/')
            .map(|j| &current_url[i + 3 + j..])?,
        None => current_url,
    };
    let path = path.split('?').next()?.split('#').next()?;
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.len() == 3 && parts[0] == "app" && parts[2] == "home" {
        Some(parts[1].to_string())
    } else {
        None
    }
}

pub async fn app_spaces(
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let spaces = state
        .spaces
        .space_repository
        .find_by_coach_id(&user.id)
        .await
        .unwrap_or_default();

    let active_space_id = headers
        .get("hx-current-url")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_space_id);

    AppSpaces {
        app_routes: AppRoutes::default(),
        spaces,
        active_space_id,
    }
    .into_response()
}
