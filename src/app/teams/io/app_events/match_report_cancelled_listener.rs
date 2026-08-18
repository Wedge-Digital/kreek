use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::teams::domain::team::Team;
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

/// Libère les 2 équipes d'un rapport annulé.
///
/// La confirmation de la sélection les avait verrouillées en
/// `GamePhase::MatchReporting`, et la seule autre sortie de cette phase est la
/// publication du rapport — qui n'aura jamais lieu. Sans ce listener, les deux
/// équipes resteraient définitivement indisponibles pour tout autre match.
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportCancelled {
                        match_report_id,
                        home_team_id,
                        away_team_id,
                        ..
                    }) = serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_cancelled(
                        &match_report_id,
                        &home_team_id,
                        &away_team_id,
                        team_repo.as_ref(),
                    )
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams::match_report_cancelled_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_cancelled(
    match_report_id: &str,
    home_team_id: &str,
    away_team_id: &str,
    repo: &dyn ITeamRepository,
) {
    let Ok(mr_id) = MatchReportId::try_new(match_report_id) else {
        tracing::warn!("teams::match_report_cancelled_listener: id invalide {match_report_id}");
        return;
    };

    for team_id in [home_team_id, away_team_id] {
        release_team(repo, team_id, mr_id).await;
    }
}

/// Chaque équipe est traitée indépendamment : l'échec de l'une ne doit pas
/// priver l'autre de sa libération.
async fn release_team(repo: &dyn ITeamRepository, team_id: &str, mr_id: MatchReportId) {
    let Some(team) = load_team(repo, team_id).await else {
        return;
    };

    // Un refus du domaine n'est pas une anomalie : une équipe qui n'est pas (ou
    // plus) en saisie sur ce rapport n'a rien à libérer. C'est aussi ce qui rend
    // le traitement idempotent quand l'événement est rejoué.
    let event = match team.cancel_match_reporting(mr_id) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!(
                "teams::match_report_cancelled_listener: rien à libérer pour {team_id}: {e}"
            );
            return;
        }
    };

    if let Err(e) = repo.append(team_id, &event, team.version).await {
        tracing::error!("teams::match_report_cancelled_listener: append {team_id}: {e}");
    }
}

async fn load_team(repo: &dyn ITeamRepository, team_id: &str) -> Option<Team> {
    match repo.find_by_id(team_id).await {
        Ok(Some(t)) => Some(t),
        Ok(None) => {
            tracing::warn!("teams::match_report_cancelled_listener: équipe {team_id} introuvable");
            None
        }
        Err(e) => {
            tracing::error!("teams::match_report_cancelled_listener: find_by_id {team_id}: {e}");
            None
        }
    }
}
