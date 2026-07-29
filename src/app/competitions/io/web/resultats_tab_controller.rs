use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::competition_detail::{full_page, load_page_base};
use crate::app::competitions::io::web::resultats_view::{
    build_journees, compute_authorization, load_resultats, JourneeResultatsVm,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TabCursorQuery {
    pub cursor: Option<i32>,
}

#[derive(Template)]
#[template(path = "competition-tab-resultats.html")]
pub struct ResultatsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

impl IntoResponse for ResultatsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_resultats_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let rows = match load_resultats(&state, &season_id, query.cursor).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let space_id_vo = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let competition_id_vo = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let authz =
        compute_authorization(&state, &user, &space_id_vo, &competition_id_vo, &season_id).await;

    let (journees, next_cursor) = build_journees(rows, 3, &authz);
    let is_htmx = headers.contains_key("hx-request");

    if is_htmx {
        return ResultatsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
            journees,
            next_cursor,
            is_initial: query.cursor.is_none(),
        }
        .into_response();
    }

    render_full_page(
        space_id,
        competition_id,
        season_id,
        journees,
        next_cursor,
        &state,
    )
    .await
}

async fn render_full_page(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeResultatsVm>,
    next_cursor: Option<i32>,
    state: &AppState,
) -> Response {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let _ = (journees, next_cursor);
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "resultats",
        false,
        vec![],
        vec![],
        vec![],
        vec![],
    )
}
