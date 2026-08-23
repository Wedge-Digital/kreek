//! L'entrée « Espace » du menu applicatif.
//!
//! C'est la seule entrée du menu qui n'est pas offerte à tous. Le masquage est
//! une **politesse** — la page rend 403 de toute façon — mais une politesse qui
//! se vérifie : sans elle, tous les membres d'un espace voient un bouton qui les
//! renverra une erreur.
//!
//! L'assertion porte sur **l'URL** d'administration, pas sur le libellé
//! « Espace » : le nom de l'espace semé est « Espace E2E », qui le contient. Une
//! assertion sur le mot passait pour l'administrateur et échouait pour l'autre,
//! en désignant la mauvaise cause.

use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::cli::seed_e2e::SIMPLE_COACH_NAME;
use crate::web::test_harness::Harnais;

async fn espace_e2e(pool: &sqlx::PgPool) -> String {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    sqlx::query_scalar::<_, String>(
        "SELECT m.space_id FROM spaces__user_space m
         JOIN auth__users u ON u.id = m.coach_id
         WHERE u.coach_name = 'DevCoach' AND m.profile = $1
         LIMIT 1",
    )
    .bind(SpaceProfile::SpaceAdmin.as_str())
    .fetch_one(pool)
    .await
    .expect("l'espace E2E où DevCoach est administrateur")
}

#[sqlx::test]
async fn un_administrateur_voit_l_entree_espace(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let menu = app
        .get_avec(
            "/app/menu",
            &[(
                "hx-current-url",
                &format!("http://localhost/app/{space_id}/home"),
            )],
        )
        .await;

    assert!(
        menu.corps.contains(&format!("/app/{space_id}/admin")),
        "l'administrateur doit voir l'entrée : {}",
        menu.corps
    );
}

/// Le pendant du précédent, et le seul des deux qui puisse échouer en silence :
/// un menu vide contient tout aussi bien « pas d'entrée Espace ». D'où
/// l'assertion sur une entrée offerte à tous, qui prouve que le menu a bien été
/// rendu pour cet espace avant qu'on constate l'absence de l'autre.
#[sqlx::test]
async fn un_membre_simple_ne_voit_pas_l_entree_espace(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let menu = app
        .get_avec(
            "/app/menu",
            &[(
                "hx-current-url",
                &format!("http://localhost/app/{space_id}/home"),
            )],
        )
        .await;

    assert!(
        menu.corps.contains("Mes équipes"),
        "le menu doit avoir été rendu pour cet espace : {}",
        menu.corps
    );
    assert!(
        !menu.corps.contains(&format!("/app/{space_id}/admin")),
        "un membre simple ne doit pas voir l'entrée d'administration : {}",
        menu.corps
    );
}
