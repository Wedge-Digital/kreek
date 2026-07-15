use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::{render_admin_page, require_admin_access};
use crate::app::competitions::io::web::resultats_view::{
    build_journees, load_resultats, JourneeResultatsVm, ResultAuthorization,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/results.html")]
pub struct ResultsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
}

impl IntoResponse for ResultsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("results tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn results_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        if let Err(resp) = require_admin_access(&auth_session, &space_id, &competition_id, &state).await {
            return resp;
        }

        let rows = match load_resultats(&state, &season_id, None).await {
            Ok(r) => r,
            Err(r) => return r,
        };
        // require_admin_access garantit déjà admin d'espace ou de
        // compétition — toutes les lignes sont autorisées ici.
        let (journees, _next_cursor) =
            build_journees(rows, usize::MAX, &ResultAuthorization::unrestricted());

        return ResultsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
            journees,
        }
        .into_response();
    }

    render_admin_page(auth_session, &space_id, &competition_id, &season_id, "results", &state).await
}
