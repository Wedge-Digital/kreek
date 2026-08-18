use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, pool: PgPool) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_event(event, &pool).instrument(span).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_confirmed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(event: MatchReportAppEvent, pool: &PgPool) {
    let MatchReportAppEvent::MatchReportConfirmed {
        match_report_id,
        space_id,
        pairing_id: Some(pairing_id),
        ..
    } = event
    else {
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .edit_match_report(&space_id, &match_report_id);

    let result = sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status = 'in_progress',
             match_report_id = $2,
             match_report_url = $3
         WHERE pairing_id = $1",
        pairing_id,
        match_report_id,
        report_url,
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!("competitions::match_report_confirmed_listener: update {pairing_id}: {e}");
    }
}
