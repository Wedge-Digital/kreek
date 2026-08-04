use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::latest_results_view::{
    compute_authorization, to_latest_result_vm, LatestResultVm,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

const MAX_RESULTS: i64 = 4;

#[derive(Template)]
#[template(path = "widgets/latest-results-widget.html")]
pub struct LatestResultsWidgetTemplate {
    pub results: Vec<LatestResultVm>,
}

impl IntoResponse for LatestResultsWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("latest_results_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn latest_results_widget(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(space_id_vo) = SpaceId::try_new(&space_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let results = build_results(&state, &user, &space_id, &space_id_vo).await;
    LatestResultsWidgetTemplate { results }.into_response()
}

/// Échec de lecture de la projection : dégradation silencieuse (widget
/// secondaire, non bloquant) — même rendu que l'état vide, log seul côté
/// serveur.
async fn build_results(
    state: &AppState,
    user: &crate::app::auth::domain::user::User,
    space_id: &str,
    space_id_vo: &SpaceId,
) -> Vec<LatestResultVm> {
    let rows = match state
        .competitions
        .match_day_repository
        .list_latest_completed_results(space_id, MAX_RESULTS)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("latest_results_widget: list_latest_completed_results: {e}");
            return vec![];
        }
    };

    let authz = compute_authorization(state, user, space_id_vo, &rows).await;
    rows.into_iter()
        .map(|r| to_latest_result_vm(r, &authz))
        .collect()
}
