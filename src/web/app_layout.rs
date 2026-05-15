use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::news::routes::Routes as NewsRoutes;
use crate::app::spaces::routes::path as spaces_path;
use crate::state::AppState;
use crate::web::routes::Routes;

#[derive(Template, Default)]
#[template(path = "app-layout.html")]
pub struct AppLayout {
    pub content: String,
    pub routes: Routes,
}

impl IntoResponse for AppLayout {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn app_layout(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return Redirect::to(crate::app::auth::routes::path::LOGIN).into_response();
    };

    let spaces = state
        .spaces.space_repository
        .find_by_coach_id(&user.id)
        .await
        .unwrap_or_default();

    let target = if let Some(first) = spaces.first() {
        NewsRoutes::default().space_home(&first.id)
    } else {
        spaces_path::SPACE_ALL.to_string()
    };

    Redirect::to(&target).into_response()
}