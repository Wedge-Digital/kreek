use crate::app::players::domain::events::PlayerDomainEvent;
use crate::common::services::event_bus::app_event_publication::publier;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;

/// Publisher du BC `players` — souscrit au bus interne (événements domaine),
/// convertit vers l'app event correspondant via
/// `PlayerDomainEvent::to_app_event()`, publie sur l'`app_event_bus`. Même
/// patron que `competitions_app_event_publisher`.
///
/// C'est le **seul** point de conversion domaine → app event du BC : un
/// listener n'émet jamais d'app event directement.
pub fn players_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    let mut rx = event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<PlayerDomainEvent>(envelope.payload.clone())
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
                    tracing::warn!("players_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
