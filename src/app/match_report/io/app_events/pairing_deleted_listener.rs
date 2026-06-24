use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(app_event_bus: &EventBus, repo: Arc<dyn IMatchReportRepository>) {
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

                    let (version, cancel_event) = match state {
                        MatchReportState::Draft(draft) => {
                            let v = draft.version;
                            let event = draft.cancel("Pairing supprimé".to_string());
                            (v, event)
                        }
                        MatchReportState::Cancelled(_) => continue,
                        _ => continue,
                    };

                    match repo.append(&mr_id, &cancel_event, version).await {
                        Ok(_) => {
                            tracing::info!(
                                "pairing_deleted_listener: cancelled match report {mr_id}"
                            );
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
