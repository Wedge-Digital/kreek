//! Le flux temps réel par le routeur de production (carte 642).
//!
//! Ce que nginx exige pour laisser passer un flux — `X-Accel-Buffering: no` —
//! se vérifie ici, sur l'en-tête **réellement émis**, et non sur le code qui
//! est censé le poser.

use crate::web::routes::path;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

/// Le compte semé par `seed_e2e`, celui qu'emploient les autres tests du harnais.
const COACH: &str = "DevCoach";

async fn connected(pool: sqlx::PgPool) -> Harnais {
    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    Harnais::connecte_en_tant_que(pool, COACH).await
}

#[sqlx::test]
async fn the_stream_carries_the_sse_headers(pool: sqlx::PgPool) {
    let harnais = connected(pool).await;

    let head = harnais.open_stream(path::REALTIME_STREAM).await;

    assert_eq!(head.status, StatusCode::OK);
    assert_eq!(head.header("content-type"), Some("text/event-stream"));
    assert_eq!(head.header("cache-control"), Some("no-cache"));
    assert_eq!(head.header("x-accel-buffering"), Some("no"));
}

#[sqlx::test]
async fn the_first_event_of_the_stream_is_hello(pool: sqlx::PgPool) {
    let harnais = connected(pool).await;

    let head = harnais.open_stream(path::REALTIME_STREAM).await;

    let frame = head.first_chunk.expect("un premier événement");
    assert!(frame.contains("event: hello"), "{frame}");
    assert!(frame.contains("retry: 5000"), "{frame}");
}

#[sqlx::test]
async fn without_a_session_the_stream_is_not_opened(pool: sqlx::PgPool) {
    let harnais = connected(pool).await;

    let head = harnais.open_stream_anonymously(path::REALTIME_STREAM).await;

    assert_eq!(head.status, StatusCode::SEE_OTHER);
    assert_ne!(head.header("content-type"), Some("text/event-stream"));
}
