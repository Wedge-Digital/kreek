use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::news::routes::Routes as NewsRoutes;
use crate::app::spaces::routes::path as spaces_path;
use crate::state::AppState;

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