use crate::app::ranking::ports::IRankingRepository;
use crate::app::ranking::use_cases::reassign_match_round_use_case;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, RoundId};
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

/// Date les lignes d'un rapport publié de sa nouvelle journée (carte 557).
/// Même forme que le listener de dépublication.
pub fn init(app_event_bus: &EventBus, repo: Arc<dyn IRankingRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportRoundReassigned {
                        match_report_id,
                        round_id,
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
                    handle_reassigned(&match_report_id, &round_id, repo.as_ref())
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        "ranking::match_report_round_reassigned_listener: lagged by {n}"
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_reassigned(match_report_id: &str, round_id: &str, repo: &dyn IRankingRepository) {
    let (Ok(mr_id), Ok(round)) = (
        MatchReportId::try_new(match_report_id),
        RoundId::try_new(round_id),
    ) else {
        tracing::warn!(
            "ranking::match_report_round_reassigned_listener: identifiant invalide ({match_report_id}, {round_id})"
        );
        return;
    };

    if let Err(e) = reassign_match_round_use_case::execute(&mr_id, &round, repo).await {
        tracing::error!(
            "ranking::match_report_round_reassigned_listener: échec pour {match_report_id}: {e:?}"
        );
    }
}
