use std::sync::{Arc, Mutex};
use crate::app::auth::domain::domain_event::AuthDomainEvent;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn auth_app_event_publisher(app_event_bus: Arc<Mutex<EventBus>>, event_bus: Arc<Mutex<EventBus>>) {
    event_bus.lock().unwrap().subscribe_all(move |event_envelope: &EventEnvelope| {
        let event_envelope = event_envelope.clone();
        let cloned_app_event_bus = Arc::clone(&app_event_bus);

        tokio::spawn(async move {
            let Ok(event) = serde_json::from_value::<AuthDomainEvent>(event_envelope.payload.clone()) else {
                return;
            };
            if let Some(app_event) = event.to_app_event() {
                cloned_app_event_bus.lock().unwrap().publish(&app_event.to_enveloppe());
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::domain::domain_event::AuthDomainEvent;
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::{CoachId, EventId};
    use crate::app::shared_kernel::email::Email;
    use crate::lib::event_envelope::EventEnvelope;

    #[tokio::test]
    async fn account_created_is_forwarded_to_app_event_bus() {
        // Given
        let event_bus     = Arc::new(Mutex::new(EventBus::new()));
        let app_event_bus = Arc::new(Mutex::new(EventBus::new()));

        let captured       = Arc::new(Mutex::new(Vec::<EventEnvelope>::new()));
        let captured_clone = Arc::clone(&captured);

        app_event_bus.lock().unwrap().subscribe_all(move |env| {
            captured_clone.lock().unwrap().push(env.clone());
        });

        auth_app_event_publisher(Arc::clone(&app_event_bus), Arc::clone(&event_bus));

        // When
        let event = AuthDomainEvent::AccountCreated {
            event_id:  EventId::new(),
            user_id:   CoachId::new(),
            user_name: CoachName::try_new("TestCoach").unwrap(),
            email:     Email::try_new("test@example.com").unwrap(),
        };
        event_bus.lock().unwrap().publish(&event.to_enveloppe());

        tokio::task::yield_now().await;

        // Then
        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type.as_str(), "AccountCreated");
    }
}

