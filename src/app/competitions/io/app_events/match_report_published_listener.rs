use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;

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
                    handle_event(event, &pool).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_published_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(event: MatchReportAppEvent, pool: &PgPool) {
    let MatchReportAppEvent::MatchReportPublished(payload) = event else {
        return;
    };
    let Some(pairing_id) = payload.pairing_id.clone() else {
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .recap(&payload.space_id, &payload.match_report_id);
    let home_cas = count_casualties(&payload.home_actions);
    let away_cas = count_casualties(&payload.away_actions);

    let result = update_projection(
        pool, &pairing_id, &payload, home_cas, away_cas, &report_url,
    )
    .await;

    if let Err(e) = result {
        tracing::error!(
            "competitions::match_report_published_listener: update {pairing_id}: {e}"
        );
    }
}

#[allow(clippy::too_many_arguments)]
async fn update_projection(
    pool: &PgPool,
    pairing_id: &str,
    payload: &MatchReportPublishedPayload,
    home_cas: i32,
    away_cas: i32,
    report_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status = 'completed',
             home_score = $2,
             away_score = $3,
             home_casualties = $4,
             away_casualties = $5,
             match_report_url = $6
         WHERE pairing_id = $1",
        pairing_id,
        payload.home_score as i32,
        payload.away_score as i32,
        home_cas,
        away_cas,
        report_url,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Compte les actions Sortie infligées par une équipe (mêmes règles que
/// `MatchReportPreMatch::compute_cas()` côté `match_report` — seule `Sortie`
/// compte comme casualty, `Blesse{..}` en est le résultat côté adverse).
fn count_casualties(actions: &[MatchActionPublishedPayload]) -> i32 {
    actions
        .iter()
        .filter(|a| matches!(a.action, ActionTypePayload::Sortie))
        .count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::app_events::match_report_app_events::PlayerRefPayload;

    fn action(action: ActionTypePayload) -> MatchActionPublishedPayload {
        MatchActionPublishedPayload {
            turn: 1,
            player: PlayerRefPayload::Regular { player_id: "p1".to_string() },
            action,
        }
    }

    #[test]
    fn count_casualties_empty_returns_zero() {
        assert_eq!(count_casualties(&[]), 0);
    }

    #[test]
    fn count_casualties_counts_sorties_only() {
        let actions = vec![
            action(ActionTypePayload::Sortie),
            action(ActionTypePayload::Sortie),
            action(ActionTypePayload::Touchdown),
        ];
        assert_eq!(count_casualties(&actions), 2);
    }

    #[test]
    fn count_casualties_ignores_blesse() {
        let actions = vec![
            action(ActionTypePayload::Blesse { injury: "Commotion".to_string() }),
            action(ActionTypePayload::Blesse { injury: "Mort".to_string() }),
        ];
        assert_eq!(count_casualties(&actions), 0);
    }
}
