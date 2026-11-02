//! Le registre des connexions : ouverture, fermeture, onglets multiples, et
//! une file pleine qui perd sans bloquer (carte 642).

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::registry::{ConnectionId, ConnectionRegistry, Delivery, QUEUE_CAPACITY};

fn coach() -> CoachName {
    CoachName::try_new("Morgane").expect("nom de coach valide")
}

fn hello(id: ConnectionId) -> RealtimeMessage {
    RealtimeMessage::Hello { connection_id: id }
}

#[test]
fn opening_registers_the_connection() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();

    let _opened = registry.open(user, coach());

    assert_eq!(registry.open_count(), 1);
    assert_eq!(registry.user_connection_count(&user), 1);
}

#[test]
fn closing_removes_the_connection_and_its_index_entry() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach());

    registry.close(&opened.id);

    assert_eq!(registry.open_count(), 0);
    assert_eq!(registry.user_connection_count(&user), 0);
    assert_eq!(
        registry.deliver(&opened.id, hello(opened.id)),
        Delivery::Gone
    );
}

#[test]
fn two_tabs_of_one_user_are_two_entries_and_closing_one_keeps_the_other() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let first = registry.open(user, coach());
    let second = registry.open(user, coach());
    assert_eq!(registry.user_connection_count(&user), 2);

    registry.close(&first.id);

    assert_eq!(registry.open_count(), 1);
    assert_eq!(registry.user_connection_count(&user), 1);
    assert_eq!(
        registry.deliver(&second.id, hello(second.id)),
        Delivery::Queued
    );
}

#[test]
fn connections_of_two_users_are_counted_apart() {
    let registry = ConnectionRegistry::new();
    let (alice, bob) = (UserId::new(), UserId::new());
    let _a = registry.open(alice, coach());
    let _b = registry.open(bob, coach());

    assert_eq!(registry.open_count(), 2);
    assert_eq!(registry.user_connection_count(&alice), 1);
    assert_eq!(registry.user_connection_count(&bob), 1);
}

#[test]
fn a_full_queue_drops_the_message_without_blocking_and_counts_it() {
    let registry = ConnectionRegistry::new();
    let opened = registry.open(UserId::new(), coach());
    for _ in 0..QUEUE_CAPACITY {
        assert_eq!(
            registry.deliver(&opened.id, hello(opened.id)),
            Delivery::Queued
        );
    }

    // Personne ne lit la file : un `send` attendrait ici indéfiniment.
    let outcome = registry.deliver(&opened.id, hello(opened.id));

    assert_eq!(outcome, Delivery::Dropped);
    assert_eq!(opened.stats.dropped(), 1);
}

#[test]
fn delivering_to_an_unknown_connection_is_gone() {
    let registry = ConnectionRegistry::new();
    let id = ConnectionId::new();

    assert_eq!(registry.deliver(&id, hello(id)), Delivery::Gone);
}

#[test]
fn a_dropped_receiver_makes_delivery_gone() {
    let registry = ConnectionRegistry::new();
    let opened = registry.open(UserId::new(), coach());
    let id = opened.id;

    drop(opened.receiver);

    assert_eq!(registry.deliver(&id, hello(id)), Delivery::Gone);
}
