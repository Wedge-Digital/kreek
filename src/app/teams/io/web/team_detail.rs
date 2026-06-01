use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::state::AppState;

/// Handler fiche d'équipe — câblage complet en carte 34.
pub async fn team_detail(
    Path((_space_id, _team_id)): Path<(String, String)>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
