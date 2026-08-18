use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::common::services::event_bus::app_event_publication::publier;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;

pub fn competitions_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    let mut rx = event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<CompetitionsDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    if let Some(app_event) = event.to_app_event() {
                        let span = tracing::info_span!(
                            "app_event_publication",
                            domain_event = %envelope.event_type,
                            cause = %envelope.event_id
                        );
                        let _garde = span.enter();
                        publier(&app_event_bus, app_event.to_enveloppe());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
