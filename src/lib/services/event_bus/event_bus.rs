use std::collections::HashMap;
use crate::lib::domain_event::DomainEventEnvelope;

type EventHandler = Box<dyn Fn(&DomainEventEnvelope) + Send + Sync>;

pub struct EventBus {
    subscribers:      HashMap<String, Vec<EventHandler>>,
    omni_subscribers: Vec<EventHandler>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers:      HashMap::new(),
            omni_subscribers: Vec::new(),
        }
    }

    pub fn subscribe_all(&mut self, handler: impl Fn(&DomainEventEnvelope) + Send + Sync + 'static) {
        self.omni_subscribers.push(Box::new(handler));
    }

    pub fn subscribe(&mut self, event_type: impl Into<String>, handler: impl Fn(&DomainEventEnvelope) + Send + Sync + 'static) {
        self.subscribers
            .entry(event_type.into())
            .or_default()
            .push(Box::new(handler));
    }

    pub fn publish(&self, event: &DomainEventEnvelope) {
        for handler in &self.omni_subscribers {
            handler(event);
        }
        if let Some(handlers) = self.subscribers.get(&event.event_type) {
            for handler in handlers {
                handler(event);
            }
        }
    }
}

pub trait IEventPublisher: Send + Sync {
    fn publish(&self, event: DomainEventEnvelope);
}

impl IEventPublisher for EventBus {
    fn publish(&self, event: DomainEventEnvelope) {
        EventBus::publish(self, &event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use time::OffsetDateTime;

    fn make_event(event_type: &str) -> DomainEventEnvelope {
        DomainEventEnvelope {
            event_id:    "01JQQQQQQQQQQQQQQQQQQQQ001".into(),
            emitter:     "01JQQQQQQQQQQQQQQQQQQQQ002".into(),
            event_type:  event_type.into(),
            tags:        serde_json::json!({}),
            payload:     serde_json::json!({}),
            occurred_at: OffsetDateTime::now_utc(),
        }
    }

    fn given_an_event_bus() -> EventBus {
        EventBus::new()
    }

    fn when_a_listener_subscribes_all_events(bus: &mut EventBus) {
        bus.subscribe_all(|_| {});
    }

    fn then_the_listener_shall_be_present_in_omni_subscribers(bus: &EventBus) {
        assert!(!bus.omni_subscribers.is_empty());
    }

    fn when_a_listener_subscribes_to_specific_event(bus: &mut EventBus) {
        bus.subscribe("UserRegistered", |_| {});
    }

    fn then_the_listener_shall_be_present_in_specific_subscribers(bus: &EventBus) {
        assert!(!bus.subscribers.is_empty());
    }

    fn when_omni_listener_subscribes_and_event_is_published(bus: &mut EventBus) -> Arc<Mutex<Vec<String>>> {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_clone = Arc::clone(&received);
        bus.subscribe_all(move |event| {
            received_clone.lock().unwrap().push(event.event_type.clone());
        });
        bus.publish(&make_event("UserRegistered"));
        received
    }

    fn when_specific_listener_subscribes_and_two_events_are_published(bus: &mut EventBus) -> Arc<Mutex<Vec<String>>> {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_clone = Arc::clone(&received);
        bus.subscribe("UserRegistered", move |event| {
            received_clone.lock().unwrap().push(event.event_type.clone());
        });
        bus.publish(&make_event("UserBanned"));
        bus.publish(&make_event("UserRegistered"));
        received
    }

    fn then_exactly_one_event_was_received(received: Arc<Mutex<Vec<String>>>) {
        assert_eq!(received.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_listener_can_subscribe_to_all_events() {
        let mut bus = given_an_event_bus();
        when_a_listener_subscribes_all_events(&mut bus);
        then_the_listener_shall_be_present_in_omni_subscribers(&bus);
    }

    #[test]
    fn an_omni_listener_receives_any_published_event() {
        let mut bus = given_an_event_bus();
        let received = when_omni_listener_subscribes_and_event_is_published(&mut bus);
        then_exactly_one_event_was_received(received);
    }

    #[test]
    fn a_listener_can_subscribe_to_a_specific_event_type() {
        let mut bus = given_an_event_bus();
        when_a_listener_subscribes_to_specific_event(&mut bus);
        then_the_listener_shall_be_present_in_specific_subscribers(&bus);
    }

    #[test]
    fn a_specific_listener_only_receives_its_subscribed_event_type() {
        let mut bus = given_an_event_bus();
        let received = when_specific_listener_subscribes_and_two_events_are_published(&mut bus);
        then_exactly_one_event_was_received(received);
    }
}