use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashSet;
use crate::app::auth::auth_backend::AuthSession;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

pub struct SpaceCard {
    pub id:        String,
    pub name:      String,
    pub logo:      String,
    pub is_member: bool,
}

#[derive(Template)]
#[template(path = "space-all.html")]
pub struct SpaceAllTemplate {
    pub spaces: Vec<SpaceCard>,
}

impl IntoResponse for SpaceAllTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn space_all(
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let (all, member_of) = tokio::try_join!(
        state.spaces.space_repository.find_all(),
        state.spaces.space_repository.find_by_coach_id(&user.id),
    ).unwrap_or_default();

    let member_ids: HashSet<String> = member_of.into_iter().map(|s| s.id).collect();

    let spaces = all
        .into_iter()
        .map(|s| SpaceCard {
            is_member: member_ids.contains(&s.id),
            id:        s.id,
            name:      s.name,
            logo:      s.logo,
        })
        .collect();

    let tmpl = SpaceAllTemplate { spaces };

    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, ..Default::default() }.into_response()
    }
}