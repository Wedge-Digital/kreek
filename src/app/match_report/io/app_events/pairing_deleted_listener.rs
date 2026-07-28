use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Annule un rapport dont le pairing vient d'être supprimé, quel que soit son
/// avancement — tant qu'il n'est pas publié.
///
/// `Published` est un cas **anormal** : la suppression d'un pairing dont le
/// rapport est publié est refusée en amont (use case de suppression). S'il
/// arrive ici, c'est que le garde-fou a été contourné, et le match est
/// désormais absent du calendrier tout en comptant encore au classement — d'où
/// le niveau `error`.
fn cancel(
    state: MatchReportState,
    mr_id: &str,
) -> Option<(u64, crate::app::match_report::domain::events::MatchReportDomainEvent)> {
    let reason = "Pairing supprimé".to_string();
    match state {
        MatchReportState::Draft(d) => Some((d.version, d.cancel(reason))),
        MatchReportState::PreMatch(pm) => Some((pm.version, pm.cancel(reason))),
        MatchReportState::ReadyToPublish(rtp) => Some((rtp.version, rtp.cancel(reason))),
        MatchReportState::Cancelled(_) => None,
        MatchReportState::Published(_) => {
            tracing::error!(
                "pairing_deleted_listener: rapport {mr_id} publié, pairing pourtant supprimé — \
                 le match reste au classement alors qu'il a disparu du calendrier"
            );
            None
        }
    }
}

/// `event_bus` est le bus **interne** du BC : l'annulation y est publiée après
/// son append, pour que le publisher la convertisse en app event. Sans ça, le
/// rapport serait annulé sans que personne ne l'apprenne — et les équipes
/// resteraient verrouillées en saisie.
pub fn init(
    app_event_bus: &EventBus,
    event_bus: &EventBus,
    repo: Arc<dyn IMatchReportRepository>,
) {
    let bus = event_bus.clone();
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<CompetitionsAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let CompetitionsAppEvent::PairingDeleted { pairing_id, .. } = app_event else {
                        continue;
                    };

                    let mr_id = match repo.find_id_by_pairing(&pairing_id).await {
                        Ok(Some(id)) => id,
                        Ok(None) => continue,
                        Err(e) => {
                            tracing::error!(
                                "pairing_deleted_listener: find_id_by_pairing {pairing_id}: {e}"
                            );
                            continue;
                        }
                    };

                    let state = match repo.find_by_id(&mr_id).await {
                        Ok(Some(s)) => s,
                        Ok(None) => continue,
                        Err(e) => {
                            tracing::error!(
                                "pairing_deleted_listener: find_by_id {mr_id}: {e}"
                            );
                            continue;
                        }
                    };

                    let Some((version, cancel_event)) = cancel(state, &mr_id) else {
                        continue;
                    };

                    match repo.append(&mr_id, &cancel_event, version).await {
                        Ok(_) => {
                            tracing::info!(
                                "pairing_deleted_listener: cancelled match report {mr_id}"
                            );
                            let _ = bus.send(cancel_event.to_enveloppe(&mr_id));
                        }
                        Err(e) => {
                            tracing::error!(
                                "pairing_deleted_listener: append cancel {mr_id}: {e}"
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("pairing_deleted_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
