use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::use_cases::cancel_match_report_use_case::{
    self, CancelMatchReportCommand, CancelMatchReportError,
};
use crate::app::match_report::use_cases::match_report_access_service::{
    is_authorized, AccesRapportDeps, PorteeRapport,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Abandonne un rapport en cours.
///
/// L'autorisation est celle du récapitulatif — admin d'espace, admin de
/// compétition, ou coach de l'une des deux équipes — et vient du **même**
/// prédicat : deux copies divergeraient, et l'écart se verrait sous la forme
/// d'un bouton offert sur une action refusée.
pub async fn post_cancel_match_report(
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

    let Some((portee, season_id)) = charger_portee(&state, &match_report_id).await else {
        tracing::warn!(match_report_id = %match_report_id, "annulation : rapport introuvable ou non annulable");
        return StatusCode::NOT_FOUND.into_response();
    };
    if !is_authorized(
        &AccesRapportDeps::from_state(&state),
        &user,
        &space_id,
        &portee,
    )
    .await
    {
        tracing::warn!(
            match_report_id = %match_report_id,
            user_id = %user.id,
            "annulation refusée : ni coach des deux équipes, ni administrateur"
        );
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = CancelMatchReportCommand {
        match_report_id: mr_id,
        cancelled_by: user.coach_name.to_string(),
    };
    match cancel_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await
    {
        Ok(()) => redirection(&space_id, &portee.competition_id, &season_id),
        Err(CancelMatchReportError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(CancelMatchReportError::NotCancellable(_)) => StatusCode::CONFLICT.into_response(),
        Err(e) => {
            tracing::error!("post_cancel_match_report: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// La portée de l'autorisation, et la saison — pour savoir où renvoyer le coach.
async fn charger_portee(
    state: &AppState,
    match_report_id: &str,
) -> Option<(PorteeRapport, String)> {
    let etat = state
        .match_report
        .match_report_repo
        .find_by_id(match_report_id)
        .await
        .ok()??;
    let season_id = match &etat {
        MatchReportState::PreMatch(pm) => pm.season_id.to_string(),
        MatchReportState::ReadyToPublish(rtp) => rtp.season_id.to_string(),
        _ => return None,
    };
    PorteeRapport::depuis_etat_annulable(&etat).map(|p| (p, season_id))
}

/// Le rapport n'existe plus : la page courante n'a plus d'objet. L'onglet des
/// résultats est l'endroit où la rencontre redevient visible — « à venir » si
/// elle était programmée, absente si elle était manuelle (carte 427).
fn redirection(space_id: &str, competition_id: &str, season_id: &str) -> Response {
    Response::builder()
        .header(
            "HX-Redirect",
            AppRoutes::default().competitions.competition_tab_resultats(
                space_id,
                competition_id,
                season_id,
            ),
        )
        .body(Body::empty())
        .unwrap()
        .into_response()
}
