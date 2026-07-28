use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::spaces::context::SpacesContext;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashSet;

pub struct SpaceCard {
    pub id: String,
    pub name: String,
    pub logo: String,
    pub is_member: bool,
}

#[derive(Template)]
#[template(path = "space-all.html")]
pub struct SpaceAllTemplate {
    pub app_routes: AppRoutes,
    pub spaces: Vec<SpaceCard>,
}

impl IntoResponse for SpaceAllTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn space_all(
    auth_session: AuthSession,
    State(ctx): State<SpacesContext>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let (all, member_of) = tokio::try_join!(
        ctx.space_repository.find_all(),
        ctx.space_repository.find_by_coach_id(&user.id),
    )
    .unwrap_or_default();

    let member_ids: HashSet<String> = member_of.into_iter().map(|s| s.id).collect();

    let spaces = all
        .into_iter()
        .map(|s| SpaceCard {
            is_member: member_ids.contains(&s.id),
            id: s.id,
            name: s.name,
            logo: s.logo.thumbnail(100, 100),
        })
        .collect();

    SpaceAllTemplate {
        app_routes: Default::default(),
        spaces,
    }
    .into_response()
}
