use crate::app::auth::domain::domain_event::AuthDomainEvent;
use crate::common::services::event_bus::app_event_publication::publier;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;

pub fn auth_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    let mut rx = event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<AuthDomainEvent>(envelope.payload.clone())
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
                    tracing::warn!("auth_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::domain::domain_event::AuthDomainEvent;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::{CoachId, EventId};
    use crate::common::services::event_bus::event_bus::new_bus;

    #[tokio::test]
    async fn account_created_is_forwarded_to_app_event_bus() {
        let event_bus = new_bus();
        let app_event_bus = new_bus();
        let mut rx = app_event_bus.subscribe();

        auth_app_event_publisher(&event_bus, app_event_bus);

        let event = AuthDomainEvent::AccountCreated {
            event_id: EventId::new(),
            user_id: CoachId::new(),
            user_name: CoachName::try_new("TestCoach").unwrap(),
            email: Email::try_new("test@example.com").unwrap(),
        };
        let _ = event_bus.send(event.to_enveloppe());

        tokio::task::yield_now().await;

        let received = rx.try_recv().unwrap();
        assert_eq!(received.event_type.as_str(), "AccountCreated");
    }
}
