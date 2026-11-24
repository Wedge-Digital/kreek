//! La réponse commune à un conflit d'écriture sur un compte rendu (carte 623).
//!
//! Deux écritures simultanées sont refusées par l'event store
//! (`RepositoryError::ConcurrentWrite`), que chaque use case d'écriture relaie
//! en `Conflict`. Cette réponse le dit au client au lieu d'un 500 muet : 409, un
//! message, et `HX-Refresh` pour que la page se recharge sur l'état qui a
//! gagné.

use crate::app::match_report::domain::error::DomainError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub const CONFLICT_MESSAGE: &str = "Le match vient d'être modifié — la page se recharge.";

/// 409, `{ "error": … }`, et `HX-Refresh: true`.
pub fn conflict_response() -> Response {
    let body = axum::Json(serde_json::json!({ "error": CONFLICT_MESSAGE }));
    (StatusCode::CONFLICT, [("HX-Refresh", "true")], body).into_response()
}

/// Le statut d'un refus du domaine : 409 pour une écriture dans un camp
/// validé (carte 659-A, § 12.5 de E22) — l'état a changé, pas la saisie —,
/// 422 pour tout autre motif.
pub fn domain_refusal_status(err: &DomainError) -> StatusCode {
    match err {
        DomainError::SideValidated(_) => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

/// Le refus du domaine, son statut et son motif.
pub fn domain_refusal_response(err: &DomainError) -> Response {
    (domain_refusal_status(err), err.to_string()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::TeamSide;

    /// Carte 659-A : un camp validé est figé, l'écriture est un conflit.
    #[test]
    fn a_write_in_a_validated_side_is_a_409() {
        let validated = DomainError::SideValidated(TeamSide::Home);
        assert_eq!(domain_refusal_status(&validated), StatusCode::CONFLICT);
        let response = domain_refusal_response(&validated);
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn any_other_domain_refusal_is_a_422() {
        let refused = DomainError::MvpAlreadyRecorded;
        assert_eq!(
            domain_refusal_status(&refused),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

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
