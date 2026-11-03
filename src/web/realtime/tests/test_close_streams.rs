//! La coupure des flux d'un utilisateur (carte 646) : le flux se termine
//! vraiment — c'est ce que voit le navigateur quand le serveur s'arrête —, les
//! autres utilisateurs gardent le leur, et les présents du match quitté
//! l'apprennent.

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::web::ports::MatchRole;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::registry::{ConnectionRegistry, Presence};
use crate::web::realtime::stream_controller::open_stream;
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;

fn coach() -> CoachName {
    CoachName::try_new("Morgane").expect("nom de coach valide")
}

#[tokio::test]
async fn closing_a_user_ends_his_stream_after_what_was_queued() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user = UserId::new();
    let mut stream = Box::pin(open_stream(registry.clone(), user, coach()));

    assert_eq!(registry.close_user_connections(&user), 1);

    // `hello` était déjà en file : il sort, puis le flux se termine.
    assert!(stream.next().await.is_some(), "hello");
    let end = tokio::time::timeout(Duration::from_secs(1), stream.next()).await;
    assert!(matches!(end, Ok(None)), "le flux doit se terminer");
    assert_eq!(registry.open_count(), 0);
}

#[test]
fn closing_a_user_closes_all_his_tabs_and_only_his() {
    let registry = ConnectionRegistry::new();
    let (user, other) = (UserId::new(), UserId::new());
    let (_tab_1, _tab_2) = (registry.open(user, coach()), registry.open(user, coach()));
    let _other_tab = registry.open(other, coach());

    assert_eq!(registry.close_user_connections(&user), 2);

    assert_eq!(registry.user_connection_count(&user), 0);
    assert_eq!(registry.user_connection_count(&other), 1);
}

#[test]
fn closing_a_user_without_tab_closes_nothing() {
    assert_eq!(
        ConnectionRegistry::new().close_user_connections(&UserId::new()),
        0
    );
}

#[test]
fn the_presents_of_the_match_left_learn_it() {
    let registry = ConnectionRegistry::new();
    let match_report_id = EntityId::new();
    let presence = Some(Presence {
        match_report_id,
        role: MatchRole::Follow,
    });
    let (user, other) = (UserId::new(), UserId::new());
    let tab = registry.open(user, coach());
    let mut other_tab = registry.open(other, coach());
    registry.set_presence(&tab.id, &user, presence).unwrap();
    registry
        .set_presence(&other_tab.id, &other, presence)
        .unwrap();

    registry.close_user_connections(&user);

    assert_eq!(
        other_tab.receiver.try_recv().ok(),
        Some(RealtimeMessage::PresenceChanged { match_report_id })
    );
}
