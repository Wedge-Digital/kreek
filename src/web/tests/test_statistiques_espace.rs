//! Les trois compteurs de l'onglet Membres.
//!
//! Le harnais vérifie ce qu'ils comptent ; **il ne peut pas vérifier qu'ils se
//! rafraîchissent** — cela dépend d'événements DOM, et seul un test de bout en
//! bout l'observe. C'est l'omission qui a coûté un aller-retour sur la liste des
//! membres, découverte par la carte 384.

use crate::cli::seed_e2e::SIMPLE_COACH_NAME;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

async fn contexte(pool: &sqlx::PgPool) -> String {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    sqlx::query_scalar(
        "SELECT m.space_id FROM spaces__user_space m
         JOIN auth__users u ON u.id = m.coach_id
         WHERE u.coach_name = 'DevCoach' AND m.profile = 'SpaceAdmin' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("l'espace administré par DevCoach")
}

fn compteur(corps: &str, nom: &str) -> String {
    let ancre = format!(r#"data-compteur="{nom}">"#);
    let i = corps.find(&ancre).unwrap_or_else(|| {
        panic!("le compteur « {nom} » doit exister : {corps}");
    }) + ancre.len();
    corps[i..]
        .split('<')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[sqlx::test]
async fn les_compteurs_derivent_de_la_meme_lecture(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app.get(&format!("/app/{space}/admin/widgets/stats")).await;

    assert_eq!(r.statut, StatusCode::OK);
    // Le seed pose DevCoach administrateur et onze membres simples.
    assert_eq!(compteur(&r.corps, "membres"), "12");
    assert_eq!(compteur(&r.corps, "administrateurs"), "1");
}

/// Le troisième compteur vaut zéro tant que les invitations n'existent pas.
///
/// Ce n'est pas un oubli : il n'y a effectivement aucune invitation en attente,
/// faute d'invitations tout court. Le test verrouille l'intention, pour qu'une
/// valeur inventée se voie.
#[sqlx::test]
async fn les_invitations_en_attente_valent_zero(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app.get(&format!("/app/{space}/admin/widgets/stats")).await;

    assert_eq!(compteur(&r.corps, "invitations"), "0");
}

/// Le compte d'administrateurs suit une promotion.
#[sqlx::test]
async fn promouvoir_un_membre_fait_monter_le_compte_d_administrateurs(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let membre: String = sqlx::query_scalar("SELECT id FROM auth__users WHERE coach_name = $1")
        .bind(SIMPLE_COACH_NAME)
        .fetch_one(&pool)
        .await
        .unwrap();
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    app.post_htmx(
        &format!("/app/{space}/admin/members/{membre}/role"),
        "profile=SpaceAdmin",
    )
    .await;

    let r = app.get(&format!("/app/{space}/admin/widgets/stats")).await;
    assert_eq!(compteur(&r.corps, "administrateurs"), "2");
    assert_eq!(compteur(&r.corps, "membres"), "12", "le total ne bouge pas");
}

#[sqlx::test]
async fn un_membre_simple_n_atteint_pas_les_statistiques(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = app.get(&format!("/app/{space}/admin/widgets/stats")).await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}
