//! La sémantique du middleware, testée **une seule fois** (carte 324).
//!
//! Les cartes 318 à 322 n'ont plus à la revérifier : elles n'apportent qu'un
//! résolveur. C'est tout l'intérêt d'un mécanisme commun — une seule définition
//! de « ce que veut dire refuser », donc une seule chose à garder juste.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

async fn espace_e2e(pool: &sqlx::PgPool) -> String {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let row: (String,) = sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
        .fetch_one(pool)
        .await
        .expect("espace E2E seedé");
    row.0
}

/// Une route sans paramètre connu du middleware doit passer — sinon la
/// migration devrait être atomique sur les huit BCs, et elle ne l'est pas.
#[sqlx::test]
async fn un_chemin_sans_ressource_connue_passe(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let reponse = app.get(&format!("/app/{space_id}/competitions")).await;

    assert_ne!(
        reponse.statut,
        StatusCode::NOT_FOUND,
        "aucun résolveur ne connaît cette route : elle ne doit pas être bloquée"
    );
}

/// Une ressource inexistante et une ressource étrangère rendent **le même**
/// `404`. C'est délibéré : un code distinct confirmerait l'existence de la
/// seconde à qui l'énumère.
#[sqlx::test]
async fn une_ressource_inexistante_rend_404_comme_une_ressource_etrangere(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let inexistante = app
        .get(&format!(
            "/app/{space_id}/players/01JZZZZZZZZZZZZZZZZZZZZZZZ/debug"
        ))
        .await;

    assert_eq!(inexistante.statut, StatusCode::NOT_FOUND);
}

/// L'espace du chemin est mal formé : la requête est invalide, il n'y a pas de
/// ressource à chercher. `400`, et non `404`.
#[sqlx::test]
async fn un_espace_mal_forme_est_un_400(pool: sqlx::PgPool) {
    espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let reponse = app
        .get("/app/pas-un-ulid/players/01JZZZZZZZZZZZZZZZZZZZZZZZ/debug")
        .await;

    assert_eq!(reponse.statut, StatusCode::BAD_REQUEST);
}

/// Le refus vaut aussi en écriture — c'est le cas qui compte, et celui que la
/// carte 316 a prouvé exploitable.
#[sqlx::test]
async fn le_refus_vaut_en_ecriture(pool: sqlx::PgPool) {
    espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let reponse = app
        .post_htmx(
            &format!(
                "/app/{AUTRE_ESPACE}/players/01JZZZZZZZZZZZZZZZZZZZZZZZ/customisation/spp/add"
            ),
            "amount=5&expected_version=0",
        )
        .await;

    assert_eq!(reponse.statut, StatusCode::NOT_FOUND);
}
