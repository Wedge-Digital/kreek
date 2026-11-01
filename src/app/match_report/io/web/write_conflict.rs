//! La réponse commune à un conflit d'écriture sur un compte rendu (carte 623).
//!
//! Deux écritures simultanées sont refusées par l'event store
//! (`RepositoryError::ConcurrentWrite`), que chaque use case d'écriture relaie
//! en `Conflict`. Cette réponse le dit au client au lieu d'un 500 muet : 409, un
//! message, et `HX-Refresh` pour que la page se recharge sur l'état qui a
//! gagné.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub const CONFLICT_MESSAGE: &str = "Le match vient d'être modifié — la page se recharge.";

/// 409, `{ "error": … }`, et `HX-Refresh: true`.
pub fn conflict_response() -> Response {
    let body = axum::Json(serde_json::json!({ "error": CONFLICT_MESSAGE }));
    (StatusCode::CONFLICT, [("HX-Refresh", "true")], body).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_conflit_rend_409() {
        assert_eq!(conflict_response().status(), StatusCode::CONFLICT);
    }

    #[test]
    fn un_conflit_recharge_la_page() {
        let response = conflict_response();
        assert_eq!(
            response.headers().get("HX-Refresh").map(|v| v.as_bytes()),
            Some(&b"true"[..])
        );
    }

    #[tokio::test]
    async fn un_conflit_dit_pourquoi_en_json() {
        let response = conflict_response();
        let content_type = response.headers().get("content-type").cloned();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(body, serde_json::json!({ "error": CONFLICT_MESSAGE }));
        assert_eq!(
            content_type.as_ref().map(|v| v.as_bytes()),
            Some(&b"application/json"[..])
        );
    }
}
