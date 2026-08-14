//! Le cloisonnement des espaces pour `news` (carte 322).
//!
//! C'est sur ce BC que l'audit (carte 316) a **prouvé l'écriture croisée** :
//! un commentaire posté sur un article d'un autre espace, depuis un espace
//! dont l'appelant était admin. La sonde avait été supprimée après coup.
//!
//! Un seul résolveur, contrairement à ce que la carte annonçait : elle
//! prévoyait un saut `comments` → `articles`, inutile puisque **aucune route
//! ne porte d'identifiant de commentaire**. Les commentaires s'atteignent par
//! `/home/articles/{article_id}/comments`, donc contrôler l'article suffit.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

async fn article_dans_l_espace_e2e(pool: &sqlx::PgPool) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E seedé");

    let (author_id,): (String,) =
        sqlx::query_as("SELECT id FROM auth__users WHERE coach_name = 'DevCoach'")
            .fetch_one(pool)
            .await
            .expect("DevCoach seedé");

    let article_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    sqlx::query(
        // `tags` est un tableau Postgres, `content` du JSONB.
        "INSERT INTO articles (id, space_id, author_id, title, abstract, tags, content)
         VALUES ($1, $2, $3, 'Article de test', 'résumé', ARRAY[]::text[], '[]'::jsonb)",
    )
    .bind(&article_id)
    .bind(&space_id)
    .bind(&author_id)
    .execute(pool)
    .await
    .expect("article de test");

    (space_id, article_id)
}

/// L'écart, prouvable en HTTP ici : l'article se sert depuis sa table, sans
/// event store — contrairement à `match_report` et `teams`.
#[sqlx::test]
async fn un_article_n_est_lisible_que_depuis_son_espace(pool: sqlx::PgPool) {
    let (space_id, article_id) = article_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let nominal = app
        .get(&format!("/app/{space_id}/home/articles/{article_id}"))
        .await;
    assert_ne!(
        nominal.statut,
        StatusCode::NOT_FOUND,
        "depuis son propre espace, l'article doit rester atteignable"
    );

    let croise = app
        .get(&format!("/app/{AUTRE_ESPACE}/home/articles/{article_id}"))
        .await;
    assert_eq!(
        croise.statut,
        StatusCode::NOT_FOUND,
        "depuis un autre espace"
    );
}

/// Le geste exact que l'audit avait réussi : commenter l'article d'un autre
/// espace. C'est la preuve d'écriture croisée qui a lancé toute la série.
#[sqlx::test]
async fn commenter_l_article_d_un_autre_espace_est_refuse(pool: sqlx::PgPool) {
    let (_, article_id) = article_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool.clone(), "DevCoach").await;

    let croise = app
        .post_htmx(
            &format!("/app/{AUTRE_ESPACE}/home/articles/{article_id}/comments"),
            "content=tentative",
        )
        .await;

    assert_eq!(croise.statut, StatusCode::NOT_FOUND);

    let (nombre,): (i64,) = sqlx::query_as("SELECT count(*) FROM comments WHERE article_id = $1")
        .bind(&article_id)
        .fetch_one(&pool)
        .await
        .expect("comptage");
    assert_eq!(nombre, 0, "un refus ne doit rien écrire");
}
