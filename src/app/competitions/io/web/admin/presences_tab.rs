//! L'onglet Présences — la coquille.
//!
//! Deux conteneurs `hx-get`, quasi zéro JS, et **la géométrie du Calendrier** :
//! les journées à gauche, le panneau à droite. Ce n'est pas de l'imitation, c'est
//! pour qu'on ne se réoriente pas d'un onglet à l'autre — l'organisateur passe de
//! l'un à l'autre en pensant à la même journée.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::{
    render_admin_page, require_admin_access,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/presences.html")]
pub struct PresencesTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}

impl IntoResponse for PresencesTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("presences tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn presences_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !crate::web::htmx::veut_la_page_entiere(&headers) {
        if let Err(resp) = require_admin_access(
            &auth_session,
            &space_id,
            &competition_id,
            &season_id,
            &state,
        )
        .await
        {
            return resp;
        }

        return PresencesTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
        }
        .into_response();
    }

    render_admin_page(
        auth_session,
        &space_id,
        &competition_id,
        &season_id,
        "presences",
        &state,
    )
    .await
}
