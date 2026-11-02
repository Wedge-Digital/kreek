//! La présence dans le registre : posée, remplacée, retirée par la déclaration
//! vide et par la fermeture, refusée sur l'onglet d'autrui, et le signal qui ne
//! va qu'aux présents (carte 644).

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::web::ports::MatchRole;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::registry::{
    ConnectionRegistry, OpenedConnection, Presence, PresenceError,
};

fn coach(name: &str) -> CoachName {
    CoachName::try_new(name).expect("nom de coach valide")
}

fn on(match_report_id: EntityId, role: MatchRole) -> Option<Presence> {
    Some(Presence {
        match_report_id,
        role,
    })
}

/// Ce qui attend dans la file de la connexion. `open` seul n'y met rien :
/// c'est le flux qui y dépose `hello`.
fn drain(opened: &mut OpenedConnection) -> Vec<RealtimeMessage> {
    let mut messages = Vec::new();
    while let Ok(message) = opened.receiver.try_recv() {
        messages.push(message);
    }
    messages
}

#[test]
fn a_presence_is_set_then_listed_on_its_match_only() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach("Morgane"));
    let (watched, other) = (EntityId::new(), EntityId::new());

    let change = registry
        .set_presence(&opened.id, &user, on(watched, MatchRole::Write))
        .unwrap();

    assert_eq!(change.matches_to_notify, vec![watched]);
    let present = registry.present_on(&watched);
    assert_eq!(present.len(), 1);
    assert_eq!(present[0].role, MatchRole::Write);
    assert_eq!(present[0].coach_name, coach("Morgane"));
    assert!(registry.present_on(&other).is_empty());
}

#[test]
fn a_new_declaration_replaces_the_previous_one_and_notifies_both_matches() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach("Morgane"));
    let (first, second) = (EntityId::new(), EntityId::new());
    registry
        .set_presence(&opened.id, &user, on(first, MatchRole::Follow))
        .unwrap();

    let change = registry
        .set_presence(&opened.id, &user, on(second, MatchRole::Follow))
        .unwrap();

    assert_eq!(change.matches_to_notify, vec![first, second]);
    assert!(registry.present_on(&first).is_empty());
    assert_eq!(registry.present_on(&second).len(), 1);
}

#[test]
fn an_empty_declaration_clears_the_presence() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach("Morgane"));
    let watched = EntityId::new();
    registry
        .set_presence(&opened.id, &user, on(watched, MatchRole::Write))
        .unwrap();

    let change = registry.set_presence(&opened.id, &user, None).unwrap();

    assert_eq!(change.matches_to_notify, vec![watched]);
    assert!(registry.present_on(&watched).is_empty());
}

#[test]
fn the_same_declaration_twice_notifies_nobody() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach("Morgane"));
    let watched = EntityId::new();
    registry
        .set_presence(&opened.id, &user, on(watched, MatchRole::Follow))
        .unwrap();

    let change = registry
        .set_presence(&opened.id, &user, on(watched, MatchRole::Follow))
        .unwrap();

    assert!(change.matches_to_notify.is_empty());
}

#[test]
fn closing_the_connection_clears_the_presence_and_returns_its_match() {
    let registry = ConnectionRegistry::new();
    let user = UserId::new();
    let opened = registry.open(user, coach("Morgane"));
    let watched = EntityId::new();
    registry
        .set_presence(&opened.id, &user, on(watched, MatchRole::Write))
        .unwrap();

    let left = registry.close(&opened.id);

    assert_eq!(left, Some(watched));
    assert!(registry.present_on(&watched).is_empty());
}

#[test]
fn closing_a_connection_without_presence_returns_nothing() {
    let registry = ConnectionRegistry::new();
    let opened = registry.open(UserId::new(), coach("Morgane"));

    assert_eq!(registry.close(&opened.id), None);
}

#[test]
fn a_presence_cannot_be_set_on_the_connection_of_another_user() {
    let registry = ConnectionRegistry::new();
    let (owner, intruder) = (UserId::new(), UserId::new());
    let opened = registry.open(owner, coach("Morgane"));
    let watched = EntityId::new();

    let outcome = registry.set_presence(&opened.id, &intruder, on(watched, MatchRole::Write));

    assert_eq!(outcome, Err(PresenceError::UnknownConnection));
    assert!(registry.present_on(&watched).is_empty());
}

#[test]
fn a_presence_on_an_unknown_connection_is_refused() {
    let registry = ConnectionRegistry::new();
    let opened = registry.open(UserId::new(), coach("Morgane"));
    let user = UserId::new();
    registry.close(&opened.id);

    let outcome = registry.set_presence(&opened.id, &user, None);

    assert_eq!(outcome, Err(PresenceError::UnknownConnection));
}

#[test]
fn the_signal_goes_to_the_present_others_only() {
    let registry = ConnectionRegistry::new();
    let (morgane, yann, tiers) = (UserId::new(), UserId::new(), UserId::new());
    let mut writer = registry.open(morgane, coach("Morgane"));
    let mut follower = registry.open(yann, coach("Yann"));
    let mut elsewhere = registry.open(tiers, coach("Tiers"));
    let (watched, other) = (EntityId::new(), EntityId::new());
    registry
        .set_presence(&writer.id, &morgane, on(watched, MatchRole::Write))
        .unwrap();
    registry
        .set_presence(&follower.id, &yann, on(watched, MatchRole::Follow))
        .unwrap();
    registry
        .set_presence(&elsewhere.id, &tiers, on(other, MatchRole::Follow))
        .unwrap();

    registry.notify_presence_changed(&watched, Some(writer.id));

    let expected = RealtimeMessage::PresenceChanged {
        match_report_id: watched,
    };
    assert_eq!(drain(&mut follower), vec![expected]);
    assert!(drain(&mut writer).is_empty(), "l'émetteur est exclu");
    assert!(
        drain(&mut elsewhere).is_empty(),
        "un autre match n'apprend rien"
    );
}
