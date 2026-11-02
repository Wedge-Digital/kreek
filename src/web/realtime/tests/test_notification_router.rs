//! Le routage des notifications : à qui va un toast, et à qui il ne va pas
//! (carte 645). Doublure du port, registre réel.

use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, TeamSidePayload,
};
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{EventId, UserId};
use crate::common::services::event_bus::event_bus::new_bus;
use crate::web::ports::IMatchAudiencePort;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::notification_router::{self, handle, on_lagged, RoutingOutcome};
use crate::web::realtime::registry::{ConnectionRegistry, OpenedConnection};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const SPACE: &str = "space-1";

/// Les équipes connues de la doublure : `(espace, équipe, coach)`. Elle borne
/// par l'espace comme l'adapter, et compte ses appels.
#[derive(Default)]
struct FakeAudience {
    teams: Vec<(String, String, UserId)>,
    calls: AtomicUsize,
}

impl FakeAudience {
    fn with(teams: &[(&str, &str, UserId)]) -> Self {
        Self {
            teams: teams
                .iter()
                .map(|(s, t, c)| (s.to_string(), t.to_string(), *c))
                .collect(),
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl IMatchAudiencePort for FakeAudience {
    async fn coaches_of_match(&self, space_id: &str, home: &str, away: &str) -> Vec<UserId> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.teams
            .iter()
            .filter(|(s, t, _)| s == space_id && (t == home || t == away))
            .map(|(_, _, c)| *c)
            .collect()
    }
}

fn coach() -> CoachName {
    CoachName::try_new("Morgane").expect("nom de coach valide")
}

fn started(match_id: &str, home: &str, away: &str, author: &UserId) -> MatchReportAppEvent {
    MatchReportAppEvent::MatchReportConfirmed {
        event_id: EventId::new(),
        match_report_id: match_id.into(),
        home_team_id: home.into(),
        away_team_id: away.into(),
        space_id: SPACE.into(),
        pairing_id: None,
        season_id: "season-1".into(),
        round_id: "round-1".into(),
        competition_id: "competition-1".into(),
        confirmed_by: author.to_string(),
        confirmed_by_name: Some("Morgane".into()),
    }
}

async fn dispatch(event: &MatchReportAppEvent, registry: &ConnectionRegistry, port: &FakeAudience) {
    handle(&event.to_enveloppe(), registry, port).await;
}

/// Ce qu'une connexion a reçu, sans attendre.
fn received(opened: &mut OpenedConnection) -> Vec<RealtimeMessage> {
    let mut messages = Vec::new();
    while let Ok(message) = opened.receiver.try_recv() {
        messages.push(message);
    }
    messages
}

fn toast_count(opened: &mut OpenedConnection) -> usize {
    received(opened)
        .iter()
        .filter(|m| matches!(m, RealtimeMessage::Toast(_)))
        .count()
}

#[tokio::test]
async fn the_author_receives_nothing_and_the_other_coach_receives_on_every_tab() {
    let (author, other) = (UserId::new(), UserId::new());
    let port = FakeAudience::with(&[(SPACE, "home", author), (SPACE, "away", other)]);
    let registry = ConnectionRegistry::new();
    let mut author_tab = registry.open(author, coach());
    let (mut tab_1, mut tab_2) = (registry.open(other, coach()), registry.open(other, coach()));

    dispatch(&started("m1", "home", "away", &author), &registry, &port).await;

    assert_eq!(toast_count(&mut author_tab), 0);
    assert_eq!(toast_count(&mut tab_1), 1);
    assert_eq!(toast_count(&mut tab_2), 1);
}

#[tokio::test]
async fn a_coach_of_another_match_receives_nothing() {
    let (author, other, stranger) = (UserId::new(), UserId::new(), UserId::new());
    let port = FakeAudience::with(&[
        (SPACE, "home", author),
        (SPACE, "away", other),
        (SPACE, "elsewhere", stranger),
    ]);
    let registry = ConnectionRegistry::new();
    let mut other_tab = registry.open(other, coach());
    let mut stranger_tab = registry.open(stranger, coach());

    dispatch(&started("m1", "home", "away", &author), &registry, &port).await;

    // Le témoin positif d'abord : sans lui, l'absence ne prouverait rien.
    assert_eq!(toast_count(&mut other_tab), 1);
    assert_eq!(toast_count(&mut stranger_tab), 0);
}

#[tokio::test]
async fn two_simultaneous_matches_stay_sealed() {
    let coaches: Vec<UserId> = (0..4).map(|_| UserId::new()).collect();
    let port = FakeAudience::with(&[
        (SPACE, "a-home", coaches[0]),
        (SPACE, "a-away", coaches[1]),
        (SPACE, "b-home", coaches[2]),
        (SPACE, "b-away", coaches[3]),
    ]);
    let registry = ConnectionRegistry::new();
    let mut tabs: Vec<OpenedConnection> =
        coaches.iter().map(|c| registry.open(*c, coach())).collect();

    dispatch(
        &started("a", "a-home", "a-away", &coaches[0]),
        &registry,
        &port,
    )
    .await;
    dispatch(
        &started("b", "b-home", "b-away", &coaches[3]),
        &registry,
        &port,
    )
    .await;

    let counts: Vec<usize> = tabs.iter_mut().map(toast_count).collect();
    assert_eq!(counts, vec![0, 1, 1, 0]);
}

#[tokio::test]
async fn a_team_of_another_space_is_not_a_recipient() {
    let (author, foreign) = (UserId::new(), UserId::new());
    let port = FakeAudience::with(&[(SPACE, "home", author), ("other-space", "away", foreign)]);
    let registry = ConnectionRegistry::new();
    let mut foreign_tab = registry.open(foreign, coach());

    dispatch(&started("m1", "home", "away", &author), &registry, &port).await;

    assert_eq!(toast_count(&mut foreign_tab), 0);
}

#[tokio::test]
async fn a_fact_without_toast_produces_none_and_never_asks_the_port() {
    let other = UserId::new();
    let port = FakeAudience::with(&[(SPACE, "away", other)]);
    let registry = ConnectionRegistry::new();
    let mut tab = registry.open(other, coach());
    let deleted = MatchReportAppEvent::MatchActionDeleted {
        event_id: EventId::new(),
        match_report_id: "m1".into(),
        space_id: SPACE.into(),
        home_team_id: "home".into(),
        away_team_id: "away".into(),
        team_side: TeamSidePayload::Home,
        deleted_by: UserId::new().to_string(),
    };

    dispatch(&deleted, &registry, &port).await;

    assert!(received(&mut tab).is_empty());
    assert_eq!(port.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn an_unreadable_author_drops_the_notification() {
    let other = UserId::new();
    let port = FakeAudience::with(&[(SPACE, "away", other)]);
    let registry = ConnectionRegistry::new();
    let mut tab = registry.open(other, coach());
    let mut event = started("m1", "home", "away", &other);
    if let MatchReportAppEvent::MatchReportConfirmed { confirmed_by, .. } = &mut event {
        *confirmed_by = "pas-un-ulid".into();
    }

    dispatch(&event, &registry, &port).await;

    assert!(received(&mut tab).is_empty());
    assert_eq!(port.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn the_outcome_counts_recipients_connected_and_delivered() {
    let (author, online, offline) = (UserId::new(), UserId::new(), UserId::new());
    let port = FakeAudience::with(&[
        (SPACE, "home", author),
        (SPACE, "away", online),
        (SPACE, "away", offline),
    ]);
    let registry = ConnectionRegistry::new();
    let (_tab_1, _tab_2) = (
        registry.open(online, coach()),
        registry.open(online, coach()),
    );
    let notice = crate::web::realtime::toast::notice_of(&started("m1", "home", "away", &author))
        .expect("un toast");

    let outcome = notification_router::route(&notice, &author, &registry, &port).await;

    assert_eq!(
        outcome,
        RoutingOutcome {
            recipients: 2,
            connected: 1,
            delivered: 2
        }
    );
}

#[test]
fn lagging_sends_resync_to_every_connection() {
    let registry = ConnectionRegistry::new();
    let mut tab_1 = registry.open(UserId::new(), coach());
    let mut tab_2 = registry.open(UserId::new(), coach());

    assert_eq!(on_lagged(12, &registry), 2);

    assert_eq!(received(&mut tab_1), vec![RealtimeMessage::Resync]);
    assert_eq!(received(&mut tab_2), vec![RealtimeMessage::Resync]);
}

/// Le câblage de bout en bout : un fait publié sur le bus arrive à l'onglet.
#[tokio::test]
async fn a_fact_published_on_the_bus_reaches_the_other_coach() {
    let (author, other) = (UserId::new(), UserId::new());
    let port = Arc::new(FakeAudience::with(&[
        (SPACE, "home", author),
        (SPACE, "away", other),
    ]));
    let registry = Arc::new(ConnectionRegistry::new());
    let mut tab = registry.open(other, coach());
    let bus = new_bus();
    notification_router::init(&bus, registry.clone(), port);

    let _ = bus.send(started("m1", "home", "away", &author).to_enveloppe()); // arch:ok test du routeur

    let message = tokio::time::timeout(Duration::from_secs(2), tab.receiver.recv())
        .await
        .expect("le toast arrive")
        .expect("la file est ouverte");
    let RealtimeMessage::Toast(toast) = message else {
        panic!("un toast était attendu : {message:?}");
    };
    assert_eq!(toast.title, "Saisie du match démarrée par Morgane");
}
