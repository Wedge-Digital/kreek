use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::use_cases::cancel_inducements_use_case::{
    self, CancelInducementsCommand, CancelInducementsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Le message d'un rapport qui n'est plus en avant-match (R3).
const NOT_IN_PRE_MATCH_MESSAGE: &str =
    "Les coups de pouce ne s'annulent plus : le match a passé l'avant-match.";

/// `POST /{id}/cancel-inducements` (carte 626) : on recommence chez le favori.
///
/// La garde `garde_saisie` du groupe de saisie a déjà vérifié le droit d'écrire.
pub async fn post_cancel_inducements(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(mr_id) = MatchReportId::try_new(&match_report_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = CancelInducementsCommand {
        match_report_id: mr_id,
        cancelled_by: user.id,
    };
    let result = cancel_inducements_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await;
    match result {
        Ok(outcome) => redirect_to_topdog(
            &space_id,
            &match_report_id,
            &outcome.topdog_team_id.to_string(),
        ),
        Err(e) => error_response(&match_report_id, e),
    }
}

fn redirect_to_topdog(space_id: &str, match_report_id: &str, topdog: &str) -> Response {
    let url = AppRoutes::default()
        .match_report
        .inducements(space_id, match_report_id, topdog);
    Response::builder()
        .header("HX-Redirect", url)
        .body(axum::body::Body::empty())
        .unwrap()
}

fn error_response(match_report_id: &str, e: CancelInducementsError) -> Response {
    if matches!(e, CancelInducementsError::Conflict) {
        return conflict_response();
    }
    match refusal(&e) {
        Some((status, message)) => {
            tracing::warn!(match_report_id, erreur = ?e, "annulation des coups de pouce refusée");
            refused(status, message)
        }
        None => {
            tracing::error!(match_report_id, erreur = ?e, "post_cancel_inducements");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Un refus de l'état du match (409) recharge la page : htmx n'échange rien
/// sur une erreur, et le clic resterait sans effet visible. Rechargé, le
/// récapitulatif montre l'état qui a refusé — sans le bouton d'annuler.
fn refused(status: StatusCode, message: String) -> Response {
    if status == StatusCode::CONFLICT {
        return (status, [("HX-Refresh", "true")], message).into_response();
    }
    (status, message).into_response()
}

/// Le statut et le message d'une annulation refusée — `None` pour une panne.
///
/// Les variantes sont nommées une par une : un joker laisserait le prochain
/// refus retomber en 500 sans un mot.
fn refusal(e: &CancelInducementsError) -> Option<(StatusCode, String)> {
    match e {
        CancelInducementsError::Domain(domain) => Some((StatusCode::CONFLICT, domain.to_string())),
        CancelInducementsError::NotInPreMatchPhase => {
            Some((StatusCode::CONFLICT, NOT_IN_PRE_MATCH_MESSAGE.to_string()))
        }
        CancelInducementsError::NotFound => Some((
            StatusCode::NOT_FOUND,
            "Compte rendu introuvable.".to_string(),
        )),
        CancelInducementsError::Conflict | CancelInducementsError::Repository(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_of_the_match_state_reloads_the_page() {
        let response = refused(StatusCode::CONFLICT, "refusé".to_string());
        assert_eq!(
            response.headers().get("HX-Refresh").map(|v| v.as_bytes()),
            Some(&b"true"[..])
        );
        let not_found = refused(StatusCode::NOT_FOUND, "introuvable".to_string());
        assert!(not_found.headers().get("HX-Refresh").is_none());
    }
    use crate::app::match_report::domain::error::DomainError;

    fn status(e: CancelInducementsError) -> Option<StatusCode> {
        refusal(&e).map(|(s, _)| s)
    }

    #[test]
    fn les_refus_metier_rendent_409() {
        for e in [
            CancelInducementsError::Domain(DomainError::MatchAlreadyStarted),
            CancelInducementsError::Domain(DomainError::NoInducementsToCancel),
            CancelInducementsError::NotInPreMatchPhase,
        ] {
            assert_eq!(status(e), Some(StatusCode::CONFLICT));
        }
    }

    #[test]
    fn une_panne_reste_une_panne() {
        assert_eq!(
            status(CancelInducementsError::Repository("base".into())),
            None
        );
    }

    #[test]
    fn un_conflit_d_ecriture_recharge_la_page() {
        let response = error_response("MR", CancelInducementsError::Conflict);
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response.headers().contains_key("HX-Refresh"));
    }

    #[test]
    fn l_annulation_renvoie_chez_le_favori() {
        let response = redirect_to_topdog("SP", "MR", "TD");
        let location = response.headers().get("HX-Redirect").unwrap();
        assert_eq!(location, "/app/SP/match-report/MR/inducements/TD");
    }
}
