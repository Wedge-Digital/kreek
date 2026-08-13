//! Le cloisonnement des espaces pour `competitions` et `ranking` (carte 318).
//!
//! La fuite avait été **prouvée** par l'audit de la carte 316 : la compétition
//! « Ligue Open » d'un espace tiers répondait `200` avec son nom réel, demandée
//! depuis un espace dont l'appelant était admin.
//!
//! La sémantique du refus est testée en carte 324 ; ici, seuls les deux
//! résolveurs — la compétition en direct, la saison par saut.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

/// Sème l'espace E2E, puis une compétition et sa saison, écrites directement :
/// le parcours de création passe par une dizaine d'écrans, et ce qu'on teste
/// ici n'en dépend pas.
async fn competition_dans_l_espace_e2e(pool: &sqlx::PgPool) -> (String, String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E seedé");

    let competition_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    let season_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();

    sqlx::query(
        "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, 'Ligue de test', '')",
    )
    .bind(&competition_id)
    .bind(&space_id)
    .execute(pool)
    .await
    .expect("compétition de test");

    sqlx::query(
        "INSERT INTO competition_seasons (id, competition_id, name, status)
         VALUES ($1, $2, 'Saison 1', 'draft')",
    )
    .bind(&season_id)
    .bind(&competition_id)
    .execute(pool)
    .await
    .expect("saison de test");

    (space_id, competition_id, season_id)
}

/// L'écart est la preuve : `200` depuis l'espace de la compétition, `404`
/// depuis un autre. Sans le résolveur, les deux rendraient `200` — c'est
/// exactement ce que l'audit a observé.
#[sqlx::test]
async fn une_competition_n_est_lisible_que_depuis_son_espace(pool: sqlx::PgPool) {
    let (space_id, competition_id, season_id) = competition_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let nominal = app
        .get(&format!(
            "/app/{space_id}/competitions/{competition_id}/{season_id}"
        ))
        .await;
    assert_ne!(
        nominal.statut,
        StatusCode::NOT_FOUND,
        "depuis son propre espace, la compétition doit être atteignable"
    );

    let croise = app
        .get(&format!(
            "/app/{AUTRE_ESPACE}/competitions/{competition_id}/{season_id}"
        ))
        .await;
    assert_eq!(
        croise.statut,
        StatusCode::NOT_FOUND,
        "depuis un autre espace"
    );
}

/// Le second résolveur, celui qui fait le saut. Une saison n'a pas d'espace en
/// propre : si la jointure vers `competitions` était fausse, ce test tomberait
/// alors que le précédent passerait.
#[sqlx::test]
async fn une_saison_herite_de_l_espace_de_sa_competition(pool: sqlx::PgPool) {
    let (space_id, competition_id, season_id) = competition_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let nominal = app
        .get(&format!(
            "/app/{space_id}/ranking/{competition_id}/{season_id}/widget"
        ))
        .await;
    assert_ne!(nominal.statut, StatusCode::NOT_FOUND);

    let croise = app
        .get(&format!(
            "/app/{AUTRE_ESPACE}/ranking/{competition_id}/{season_id}/widget"
        ))
        .await;
    assert_eq!(
        croise.statut,
        StatusCode::NOT_FOUND,
        "`ranking` est couvert par les résolveurs de `competitions`, sans rien apporter"
    );
}

/// Une saison **d'une autre compétition** est aussi illicite qu'une compétition
/// d'un autre espace — et c'est le fait d'avoir deux résolveurs, et non un
/// seul, qui l'attrape.
#[sqlx::test]
async fn une_saison_d_un_autre_espace_est_refusee_meme_avec_une_competition_valide(
    pool: sqlx::PgPool,
) {
    let (space_id, competition_id, _) = competition_dans_l_espace_e2e(&pool).await;

    // Une seconde compétition, dans un espace différent, avec sa saison.
    let autre_comp = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    let autre_saison = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    sqlx::query("INSERT INTO spaces (id, space_name, space_icon_path) VALUES ($1, 'Ailleurs', '')")
        .bind(AUTRE_ESPACE)
        .execute(&pool)
        .await
        .expect("second espace");
    sqlx::query(
        "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, 'Ailleurs', '')",
    )
    .bind(&autre_comp)
    .bind(AUTRE_ESPACE)
    .execute(&pool)
    .await
    .expect("compétition ailleurs");
    sqlx::query(
        "INSERT INTO competition_seasons (id, competition_id, name, status)
         VALUES ($1, $2, 'S1', 'draft')",
    )
    .bind(&autre_saison)
    .bind(&autre_comp)
    .execute(&pool)
    .await
    .expect("saison ailleurs");

    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    // Compétition légitime, saison volée : le résolveur saison doit refuser.
    let reponse = app
        .get(&format!(
            "/app/{space_id}/competitions/{competition_id}/{autre_saison}"
        ))
        .await;

    assert_eq!(reponse.statut, StatusCode::NOT_FOUND);
}
