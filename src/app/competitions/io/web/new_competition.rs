use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::competitions::routes::Routes;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

#[derive(Template, Default)]
#[template(path = "new-competition-phase-1.html")]
pub struct NewCompetitionTemplate {
    pub space_id:        String,
    pub logo_url_value:  String,
    pub logo_error:      Option<String>,
    pub competition_routes: Routes,
}

impl IntoResponse for NewCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_1(
    Path(space_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let tmpl = NewCompetitionTemplate { space_id, ..Default::default() };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}

// ── Fragment: liste des membres pour le widget admins ────────────────────────

pub struct MemberItem {
    pub id:   String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "competition-members-widget.html")]
pub struct MembersWidgetTemplate {
    pub members: Vec<MemberItem>,
}

impl IntoResponse for MembersWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_members_widget(
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cached = state
        .competitions
        .competitions_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let members = cached
        .into_iter()
        .map(|u| MemberItem {
            id:   u.id.to_string(),
            name: u.coach_name.into_inner(),
        })
        .collect();

    MembersWidgetTemplate { members }.into_response()
}