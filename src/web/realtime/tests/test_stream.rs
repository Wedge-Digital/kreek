//! Le flux d'un onglet : `hello` d'abord, et la sortie du registre au `Drop`
//! (carte 642). Sans base ni HTTP : le stream est éprouvé directement.

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::web::realtime::registry::ConnectionRegistry;
use crate::web::realtime::stream_controller::open_stream;
use axum::response::sse::Sse;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::sync::Arc;

fn coach() -> CoachName {
    CoachName::try_new("Morgane").expect("nom de coach valide")
}

/// Le premier bloc du corps SSE, tel qu'axum l'écrit.
async fn first_frame(registry: Arc<ConnectionRegistry>) -> String {
    let stream = open_stream(registry, UserId::new(), coach());
    let mut body = Sse::new(stream)
        .into_response()
        .into_body()
        .into_data_stream();
    let chunk = body
        .next()
        .await
        .expect("un premier bloc")
        .expect("lisible");
    String::from_utf8_lossy(&chunk).into_owned()
}

#[tokio::test]
async fn the_first_event_is_hello_with_the_connection_id_and_retry() {
    let frame = first_frame(Arc::new(ConnectionRegistry::new())).await;

    assert!(frame.contains("event: hello"), "{frame}");
    assert!(frame.contains("retry: 5000"), "{frame}");
    assert!(frame.contains("\"connection_id\":\""), "{frame}");
}

#[tokio::test]
async fn opening_the_stream_registers_the_connection() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user = UserId::new();

    let _stream = open_stream(registry.clone(), user, coach());

    assert_eq!(registry.open_count(), 1);
    assert_eq!(registry.user_connection_count(&user), 1);
}

#[tokio::test]
async fn dropping_the_stream_removes_the_connection() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user = UserId::new();
    let stream = open_stream(registry.clone(), user, coach());

    drop(stream);

    assert_eq!(registry.open_count(), 0);
    assert_eq!(registry.user_connection_count(&user), 0);
}

#[tokio::test]
async fn dropping_one_tab_keeps_the_other_open() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user = UserId::new();
    let first = open_stream(registry.clone(), user, coach());
    let _second = open_stream(registry.clone(), user, coach());

    drop(first);

    assert_eq!(registry.open_count(), 1);
    assert_eq!(registry.user_connection_count(&user), 1);
}
