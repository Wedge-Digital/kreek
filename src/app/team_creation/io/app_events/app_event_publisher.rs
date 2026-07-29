use crate::app::team_creation::domain_event::TeamCreationDomainEvent;
use crate::common::services::event_bus::event_bus::EventBus;

pub fn team_creation_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<TeamCreationDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    if let Some(app_event) = event.to_app_event() {
                        tracing::info!(
                            "team_creation_app_event_publisher: relaying {} to app bus",
                            event.to_event_type()
                        );
                        let _ = app_event_bus.send(app_event.to_enveloppe());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("team_creation_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
