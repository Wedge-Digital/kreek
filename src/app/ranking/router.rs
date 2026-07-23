use crate::state::AppState;
use axum::Router;

/// Route du widget Classement branchée en carte 197.
pub fn router() -> Router<AppState> {
    Router::new()
}
