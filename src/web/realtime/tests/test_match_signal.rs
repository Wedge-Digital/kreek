//! Le direct (carte 646) : tout fait de match signale « le match X a changé »
//! aux connexions présentes sur ce match, auteur compris, et à elles seules.
//! Doublure du port, registre réel.

use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, TeamSidePayload,
};
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{EntityId, EventId, UserId};
use crate::common::services::event_bus::event_bus::new_bus;
use crate::web::ports::{IMatchAudiencePort, MatchRole};
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::notification_router::{self, handle};
use crate::web::realtime::registry::{ConnectionRegistry, OpenedConnection, Presence};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Une audience vide qui compte ses appels : le signal ne doit jamais la
/// solliciter.
#[derive(Default)]
struct CountingAudience {
    calls: AtomicUsize,
}

#[async_trait]
impl IMatchAudiencePort for CountingAudience {
    async fn coaches_of_match(&self, _: &str, _: &str, _: &str) -> Vec<UserId> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Vec::new()
    }
}

fn coach() -> CoachName {
    CoachName::try_new("Morgane").expect("nom de coach valide")
}

/// Un onglet ouvert et présent sur `match_report_id` avec ce rôle.
fn present_tab(
    registry: &ConnectionRegistry,
    match_report_id: EntityId,
    role: MatchRole,
) -> OpenedConnection {
    let user_id = UserId::new();
    let opened = registry.open(user_id, coach());
    let presence = Some(Presence {
        match_report_id,
        role,
    });
    registry
        .set_presence(&opened.id, &user_id, presence)
        .expect("l'onglet est à cet utilisateur");
    opened
}

/// Le retrait d'une action : un fait qui ne fait pas de toast (décision 7).
fn deleted(match_report_id: &EntityId, by: &UserId) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchActionDeleted {
        event_id: EventId::new(),
        match_report_id: match_report_id.to_string(),
        space_id: "space-1".into(),
        home_team_id: "home".into(),
        away_team_id: "away".into(),
        team_side: TeamSidePayload::Home,
        deleted_by: by.to_string(),
    }
}

fn received(opened: &mut OpenedConnection) -> Vec<RealtimeMessage> {
    let mut messages = Vec::new();
    while let Ok(message) = opened.receiver.try_recv() {
        messages.push(message);
    }
    messages
}

fn changed(match_report_id: EntityId) -> RealtimeMessage {
    RealtimeMessage::MatchChanged { match_report_id }
}

#[tokio::test]
async fn a_match_fact_signals_every_connection_present_on_that_match() {
    let registry = ConnectionRegistry::new();
    let match_id = EntityId::new();
    let mut follower = present_tab(&registry, match_id, MatchRole::Follow);
    // L'onglet de saisie de l'auteur aussi : « auteur compris ».
    let mut writer = present_tab(&registry, match_id, MatchRole::Write);

    handle(
        &deleted(&match_id, &UserId::new()).to_enveloppe(),
        &registry,
        &CountingAudience::default(),
    )
    .await;

    assert_eq!(received(&mut follower), vec![changed(match_id)]);
    assert_eq!(received(&mut writer), vec![changed(match_id)]);
}

#[tokio::test]
async fn a_signal_never_reaches_the_connections_present_on_another_match() {
    let registry = ConnectionRegistry::new();
    let (this_match, other_match) = (EntityId::new(), EntityId::new());
    let mut witness = present_tab(&registry, this_match, MatchRole::Follow);
    let mut elsewhere = present_tab(&registry, other_match, MatchRole::Follow);

    handle(
        &deleted(&this_match, &UserId::new()).to_enveloppe(),
        &registry,
        &CountingAudience::default(),
    )
    .await;

    // Le témoin d'abord : sans lui, un signal jamais livré passerait.
    assert_eq!(received(&mut witness), vec![changed(this_match)]);
    assert!(received(&mut elsewhere).is_empty());
}

#[tokio::test]
async fn a_connection_without_presence_receives_no_signal() {
    let registry = ConnectionRegistry::new();
    let match_id = EntityId::new();
    let mut witness = present_tab(&registry, match_id, MatchRole::Follow);
    let mut elsewhere_in_the_app = registry.open(UserId::new(), coach());

    handle(
        &deleted(&match_id, &UserId::new()).to_enveloppe(),
        &registry,
        &CountingAudience::default(),
    )
    .await;

    assert_eq!(received(&mut witness), vec![changed(match_id)]);
    assert!(received(&mut elsewhere_in_the_app).is_empty());
}

#[tokio::test]
async fn the_signal_never_asks_the_audience_port() {
    let registry = ConnectionRegistry::new();
    let match_id = EntityId::new();
    let mut follower = present_tab(&registry, match_id, MatchRole::Follow);
    let audience = CountingAudience::default();

    handle(
        &deleted(&match_id, &UserId::new()).to_enveloppe(),
        &registry,
        &audience,
    )
    .await;

    assert_eq!(received(&mut follower), vec![changed(match_id)]);
    assert_eq!(audience.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn the_signal_returns_the_number_of_queued_connections() {
    let registry = ConnectionRegistry::new();
    let match_id = EntityId::new();
    let _tabs = (
        present_tab(&registry, match_id, MatchRole::Follow),
        present_tab(&registry, match_id, MatchRole::Write),
        present_tab(&registry, EntityId::new(), MatchRole::Follow),
    );
    let event = deleted(&match_id, &UserId::new());

    let queued = notification_router::signal(&event, &event.to_enveloppe(), &registry);

    assert_eq!(queued, 2);
}

/// Un identifiant de match illisible n'est livré à personne — et ne fait pas
/// paniquer le routeur.
#[test]
fn an_unreadable_match_id_signals_nobody() {
    let registry = ConnectionRegistry::new();
    let mut follower = present_tab(&registry, EntityId::new(), MatchRole::Follow);
    let event = MatchReportAppEvent::MatchActionDeleted {
        event_id: EventId::new(),
        match_report_id: "pas-un-ulid".into(),
        space_id: "space-1".into(),
        home_team_id: "home".into(),
        away_team_id: "away".into(),
        team_side: TeamSidePayload::Away,
        deleted_by: UserId::new().to_string(),
    };

    assert_eq!(
        notification_router::signal(&event, &event.to_enveloppe(), &registry),
        0
    );
    assert!(received(&mut follower).is_empty());
}

#[test]
fn the_match_changed_message_is_a_match_event_with_its_id() {
    let match_id = EntityId::new();
    let rendered = format!("{:?}", changed(match_id).to_sse_event());
    assert!(rendered.contains("match"), "{rendered}");
    assert!(rendered.contains(&match_id.to_string()), "{rendered}");
}

/// Le câblage de bout en bout : un fait publié sur le bus signale l'onglet
/// présent sur son match.
#[tokio::test]
async fn a_fact_published_on_the_bus_signals_the_present_follower() {
    let registry = Arc::new(ConnectionRegistry::new());
    let match_id = EntityId::new();
    let mut follower = present_tab(&registry, match_id, MatchRole::Follow);
    let bus = new_bus();
    notification_router::init(
        &bus,
        registry.clone(),
        Arc::new(CountingAudience::default()),
    );

    let _ = bus.send(deleted(&match_id, &UserId::new()).to_enveloppe()); // arch:ok test du routeur

    let message = tokio::time::timeout(Duration::from_secs(2), follower.receiver.recv())
        .await
        .expect("le signal arrive")
        .expect("la file est ouverte");
    assert_eq!(message, changed(match_id));
}
