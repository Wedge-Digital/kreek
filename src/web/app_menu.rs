use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::state::AppState;
use crate::web::routes::Routes;

#[derive(Template, Default)]
#[template(path = "app-menu.html")]
pub struct AppMenu {
    pub routes:     Routes,
    pub space_name: Option<String>,
}

impl IntoResponse for AppMenu {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn extract_space_id(current_url: &str) -> Option<String> {
    let path = match current_url.find("://") {
        Some(i) => current_url[i + 3..].find('/').map(|j| &current_url[i + 3 + j..])?,
        None    => current_url,
    };
    let path = path.split('?').next()?.split('#').next()?;
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.len() == 3 && parts[0] == "app" && parts[2] == "home" {
        Some(parts[1].to_string())
    } else {
        None
    }
}

pub async fn app_menu(
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let space_name = async {
        let user = auth_session.user?;
        let space_id = headers.get("hx-current-url")
            .and_then(|v| v.to_str().ok())
            .and_then(extract_space_id)?;
        let spaces = state.space_repository.find_by_coach_id(&user.id).await.ok()?;
        spaces.into_iter().find(|s| s.id == space_id).map(|s| s.name)
    }.await;

    AppMenu { space_name, ..Default::default() }.into_response()
}