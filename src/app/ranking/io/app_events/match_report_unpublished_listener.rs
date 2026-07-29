use crate::app::ranking::ports::IRankingRepository;
use crate::app::ranking::use_cases::revert_match_ranking_use_case;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Retire du classement les lignes d'un rapport dépublié pour correction. Le
/// rejeu à la re-publication réinsérera les lignes recalculées.
pub fn init(app_event_bus: &EventBus, repo: Arc<dyn IRankingRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportUnpublished(payload)) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    handle_unpublished(&payload.match_report_id, repo.as_ref()).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("ranking::match_report_unpublished_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_unpublished(match_report_id: &str, repo: &dyn IRankingRepository) {
    let Ok(mr_id) = MatchReportId::try_new(match_report_id) else {
        tracing::warn!("ranking::match_report_unpublished_listener: id invalide {match_report_id}");
        return;
    };

    if let Err(e) = revert_match_ranking_use_case::execute(&mr_id, repo).await {
        tracing::error!(
            "ranking::match_report_unpublished_listener: échec pour {match_report_id}: {e:?}"
        );
    }
}
