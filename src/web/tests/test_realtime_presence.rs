//! La présence par le routeur de production (carte 644) : ce qui se refuse
//! sans navigateur.
//!
//! Le harnais referme le flux après son premier bloc : une connexion qu'il a
//! ouverte n'existe plus au moment du `POST`. La déclaration acceptée, et le
//! widget qui la montre, se vérifient donc en e2e
//! (`tests/e2e/test_realtime_presence.py`).

use crate::app::shared_kernel::identity::ids::EntityId;
use crate::web::realtime::registry::ConnectionId;
use crate::web::routes::{path, Routes};
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

async fn connected(pool: &sqlx::PgPool) -> (Harnais, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let space: String = sqlx::query_scalar(
        "SELECT m.space_id FROM spaces__user_space m
         JOIN auth__users u ON u.id = m.coach_id
         WHERE u.coach_name = 'DevCoach' AND m.profile = 'SpaceAdmin' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("l'espace administré par DevCoach");
    (
        Harnais::connecte_en_tant_que(pool.clone(), "DevCoach").await,
        space,
    )
}

#[sqlx::test]
async fn an_unknown_connection_is_not_found(pool: sqlx::PgPool) {
    let (harnais, _) = connected(&pool).await;
    let body = format!("connection_id={}", ConnectionId::new());

    let response = harnais.post_htmx(path::REALTIME_PRESENCE, &body).await;

    assert_eq!(response.statut, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn an_unreadable_declaration_is_a_bad_request(pool: sqlx::PgPool) {
    let (harnais, space) = connected(&pool).await;
    let body = format!(
        "connection_id={}&space_id={space}&match_report_id={}&activity=admin",
        ConnectionId::new(),
        EntityId::new()
    );

    let response = harnais.post_htmx(path::REALTIME_PRESENCE, &body).await;

    assert_eq!(response.statut, StatusCode::BAD_REQUEST);
}

/// `space_scope` refuse un match qui n'est pas de l'espace, avant même que
/// le widget ne demande le rôle.
#[sqlx::test]
async fn the_widget_of_an_unknown_match_is_not_found(pool: sqlx::PgPool) {
    let (harnais, space) = connected(&pool).await;
    let url = Routes.realtime_match_presence(&space, &EntityId::new().to_string());

    let response = harnais.get(&url).await;

    assert_eq!(response.statut, StatusCode::NOT_FOUND);
}
