use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

pub trait Event: Display {
    type Kind: Hash + Eq + Clone + Display;
    fn kind(&self) -> Self::Kind;
}

type EventHandler<E> = Box<dyn Fn(&E) + Send + Sync>;

pub struct EventBus<E: Event> {
    suscribers: HashMap<E::Kind, Vec<EventHandler<E>>>,
    pub omni_suscribers: Vec<EventHandler<E>>,
}

impl<E: Event> EventBus<E> {
    pub fn new() -> Self {
        Self {
            suscribers: HashMap::new(),
            omni_suscribers: Vec::new(),
        }
    }

    pub fn suscribe_all(&mut self, listener: impl Fn(&E) + Send + Sync + 'static) {
        self.omni_suscribers.push(Box::new(listener));
    }

    pub fn suscribe(&mut self, listener: impl Fn(&E) + Send + Sync + 'static, event_type: E::Kind) {
        self.suscribers.entry(event_type).or_insert_with(Vec::new).push(Box::new(listener));
    }

    pub fn publish(&self, event: &E) {
        for listener in &self.omni_suscribers {
            listener(event);
        }
        if let Some(handlers) = self.suscribers.get(&event.kind()) {
            for listener in handlers {
                listener(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::app::shared_kernel::common_types::EventId;

    #[derive(Eq, Hash, PartialEq, Clone, Debug)]
    enum TestEventKind {
        Dummy,
        AnotherDummy,
    }

    impl std::fmt::Display for TestEventKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestEventKind::Dummy       => write!(f, "Dummy"),
                TestEventKind::AnotherDummy => write!(f, "AnotherDummy"),
            }
        }
    }

    #[derive(Debug)]
    enum TestEvent {
        Dummy { event_id: EventId, content: String },
        AnotherDummy { event_id: EventId, content: String },
    }

    impl std::fmt::Display for TestEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.kind())
        }
    }

    impl Event for TestEvent {
        type Kind = TestEventKind;
        fn kind(&self) -> TestEventKind {
            match self {
                TestEvent::Dummy { .. } => TestEventKind::Dummy,
                TestEvent::AnotherDummy { .. } => TestEventKind::AnotherDummy,
            }
        }
    }

    fn given_an_event_bus() -> EventBus<TestEvent> {
        EventBus::new()
    }

    fn when_a_listener_suscribes_all_events(bus: &mut EventBus<TestEvent>) {
        bus.suscribe_all(|_event| {});
    }

    fn then_the_listener_shall_be_present_in_omnisuscribers_list(bus: &EventBus<TestEvent>) {
        assert!(!bus.omni_suscribers.is_empty());
    }

    fn when_listener_suscribes_a_specific_event(bus: &mut EventBus<TestEvent>) {
        bus.suscribe(|_event| {}, TestEventKind::Dummy);
    }

    fn then_the_listener_shall_be_present_in_specific_suscribers_list(bus: &EventBus<TestEvent>) {
        assert!(!bus.suscribers.is_empty());
    }

    fn when_an_omni_listener_suscribes_and_an_event_is_published(bus: &mut EventBus<TestEvent>) -> Arc<Mutex<Vec<String>>> {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_clone = Arc::clone(&received);
        bus.suscribe_all(move |event| {
            received_clone.lock().unwrap().push(format!("{:?}", event));
        });
        bus.publish(&TestEvent::Dummy { event_id: EventId::new(), content: String::from("an event content") });
        received
    }

    fn when_a_specific_listener_suscribes_and_both_event_kinds_are_published(bus: &mut EventBus<TestEvent>) -> Arc<Mutex<Vec<String>>> {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_clone = Arc::clone(&received);
        bus.suscribe(move |event| {
            received_clone.lock().unwrap().push(format!("{:?}", event));
        }, TestEventKind::Dummy);
        bus.publish(&TestEvent::AnotherDummy { event_id: EventId::new(), content: String::from("ignored") });
        bus.publish(&TestEvent::Dummy { event_id: EventId::new(), content: String::from("received") });
        received
    }

    fn then_the_listener_received_exactly_one_event(received: Arc<Mutex<Vec<String>>>) {
        assert_eq!(received.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_event_listener_shall_be_able_to_register_to_all_events() {
        let mut bus = given_an_event_bus();
        when_a_listener_suscribes_all_events(&mut bus);
        then_the_listener_shall_be_present_in_omnisuscribers_list(&bus);
    }

    #[test]
    fn a_registered_listener_in_omnisuscribers_shall_receive_any_event() {
        let mut bus = given_an_event_bus();
        let received = when_an_omni_listener_suscribes_and_an_event_is_published(&mut bus);
        then_the_listener_received_exactly_one_event(received);
    }

    #[test]
    fn a_event_listener_shall_be_able_to_register_to_specific_events() {
        let mut bus = given_an_event_bus();
        when_listener_suscribes_a_specific_event(&mut bus);
        then_the_listener_shall_be_present_in_specific_suscribers_list(&bus);
    }

    #[test]
    fn a_specific_listener_shall_only_receive_its_subscribed_event() {
        let mut bus = given_an_event_bus();
        let received = when_a_specific_listener_suscribes_and_both_event_kinds_are_published(&mut bus);
        then_the_listener_received_exactly_one_event(received);
    }
}