//! La liste des candidats à l'ajout direct.
//!
//! Deux tests portent tout le reste : le seuil s'applique **avant** la lecture,
//! et les trois états sont bien trois.

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

/// Le seuil s'applique avant la lecture.
///
/// Un `q` d'un caractère ne doit **rien** chercher : un seuil qui filtrerait le
/// résultat aurait déjà interrogé l'annuaire, et le garde-fou serait décoratif.
/// L'observable est l'état rendu, qui ne dit rien d'une recherche.
#[sqlx::test]
async fn un_seul_caractere_rend_l_etat_sous_seuil(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/candidates?q=D"))
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("au moins deux caractères"), "{}", r.corps);
    assert!(
        !r.corps.contains("Créez-lui un compte"),
        "l'état sous-seuil ne propose pas de créer un compte : {}",
        r.corps
    );
    assert!(!r.corps.contains("sac-row"), "aucun candidat n'est rendu");
}

/// L'état vide, lui, propose la création — et c'est ce qui le distingue.
#[sqlx::test]
async fn une_recherche_sans_resultat_propose_de_creer_un_compte(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q=ZzzPersonneIci"
        ))
        .await;

    assert!(r.corps.contains("Créez-lui un compte"), "{}", r.corps);
    assert!(
        r.corps.contains("ZzzPersonneIci"),
        "la requête est rappelée"
    );
    assert!(!r.corps.contains("au moins deux caractères"));
}

#[sqlx::test]
async fn un_membre_de_l_espace_est_rendu_avec_son_badge(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q={}",
            SIMPLE_COACH_NAME.replace(' ', "+")
        ))
        .await;

    assert!(r.corps.contains("Déjà membre"), "{}", r.corps);
    assert!(
        !r.corps.contains("sac-btn"),
        "une ligne « déjà membre » ne porte ni bouton ni sélecteur : {}",
        r.corps
    );
}

#[sqlx::test]
async fn un_non_membre_est_rendu_avec_son_selecteur_et_son_bouton(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    // Un coach de la plateforme qui n'est membre d'aucun espace.
    sqlx::query(
        "INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email)
         VALUES ('01JSOLITAIRE00000000000000', 'Solitaire', NULL, 'solitaire@bb.club')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q=Solitaire"
        ))
        .await;

    assert!(r.corps.contains("sac-btn"), "{}", r.corps);
    assert!(r.corps.contains("kreek-select"));
    assert!(!r.corps.contains("Déjà membre"));
}

#[sqlx::test]
async fn un_membre_simple_n_atteint_pas_la_liste_des_candidats(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/candidates?q=Dev"))
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}
