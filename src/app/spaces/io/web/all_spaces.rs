use crate::app::auth::auth_backend::AuthSession;
use crate::app::spaces::routes::Routes;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::host_layout::render_page;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
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
    pub routes: Routes,
    pub content_target: String,
    pub spaces: Vec<SpaceCard>,
}

pub async fn space_all(
    auth_session: AuthSession,
    State(ctx): State<SpacesContext>,
    headers: HeaderMap,
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

    let page = SpaceAllTemplate {
        routes: Routes,
        content_target: ctx.host_layout.content_target(),
        spaces,
    };
    render_page(page, &headers, ctx.host_layout.as_ref())
}
