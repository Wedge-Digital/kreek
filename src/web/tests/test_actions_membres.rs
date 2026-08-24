//! Les deux actions de la ligne de membre, frappées **sans passer par
//! l'interface**.
//!
//! C'est tout l'objet de ces tests. Le widget grise le sélecteur du dernier
//! administrateur et retire le bouton de sa propre ligne — mais ce grisage est
//! une politesse, pas une garde. Ces endpoints sont directement atteignables, et
//! doivent refuser d'eux-mêmes.

use crate::cli::seed_e2e::SIMPLE_COACH_NAME;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

/// Un espace où l'appelant n'est membre d'aucune façon.
const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

async fn contexte(pool: &sqlx::PgPool) -> (String, String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let space: String = sqlx::query_scalar(
        "SELECT m.space_id FROM spaces__user_space m
         JOIN auth__users u ON u.id = m.coach_id
         WHERE u.coach_name = 'DevCoach' AND m.profile = 'SpaceAdmin' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("l'espace administré par DevCoach");
    let admin: String = id_de(pool, "DevCoach").await;
    let membre: String = id_de(pool, SIMPLE_COACH_NAME).await;
    (space, admin, membre)
}

async fn id_de(pool: &sqlx::PgPool, nom: &str) -> String {
    sqlx::query_scalar("SELECT id FROM auth__users WHERE coach_name = $1")
        .bind(nom)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| panic!("le coach « {nom} » doit être semé"))
}

// ── Le grisage n'est pas la garde ───────────────────────────────────────────

/// Le widget ne rend pas de bouton de retrait sur sa propre ligne. Un POST
/// direct doit être refusé quand même — et par le **domaine**, pas par la
/// couche web : c'est `ActeurEstLaCible` qui répond, traduit en 403.
#[sqlx::test]
async fn un_administrateur_ne_peut_pas_se_retirer_lui_meme(pool: sqlx::PgPool) {
    let (space, admin, _) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(&format!("/app/{space}/admin/members/{admin}/remove"), "")
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
    assert!(
        r.corps.contains("votre propre ligne"),
        "le refus doit venir du domaine, avec son message : {}",
        r.corps
    );
}

#[sqlx::test]
async fn un_administrateur_ne_peut_pas_changer_son_propre_role(pool: sqlx::PgPool) {
    let (space, admin, _) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/{admin}/role"),
            "profile=SpaceUser",
        )
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}

/// L'espace ne peut pas perdre son dernier administrateur — et il ne le peut pas
/// **par construction**, sans que `DernierAdministrateur` ait à se déclencher.
///
/// Atteindre cette erreur depuis le web demanderait un acteur administrateur,
/// distinct de la cible, et une cible seule administratrice : trois conditions
/// contradictoires. Si la cible est seule, l'acteur distinct n'est pas
/// administrateur, et `is_admin()` l'arrête avant le use case.
///
/// La règle du domaine reste — elle protège les autres appelants, et les tests
/// d'agrégat la couvrent. Ce test-ci constate que le seul chemin web imaginable
/// est fermé plus tôt, et il échouera si quelqu'un desserre `is_admin()`.
#[sqlx::test]
async fn le_dernier_administrateur_est_protege_avant_meme_la_regle(pool: sqlx::PgPool) {
    let (space, admin, _) = contexte(&pool).await;
    let simple = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = simple
        .post_htmx(
            &format!("/app/{space}/admin/members/{admin}/role"),
            "profile=SpaceUser",
        )
        .await;

    assert_eq!(
        r.statut,
        StatusCode::FORBIDDEN,
        "un membre simple n'atteint pas l'action, donc jamais la règle"
    );
}

// ── La matrice d'autorisation ───────────────────────────────────────────────

/// La cible est un **autre** coach que l'appelant, et ce détail décide de tout.
///
/// Une première version visait l'appelant lui-même : `ActeurEstLaCible` rendait
/// alors 403 à la place de la garde, et le test passait même sans `is_admin()`.
/// Il passait pour la mauvaise raison — ce qui est pire qu'échouer.
#[sqlx::test]
async fn un_membre_simple_n_atteint_aucun_endpoint_d_administration(pool: sqlx::PgPool) {
    let (space, _, _) = contexte(&pool).await;
    let autre = id_de(&pool, "E2E Coach 02").await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    for uri in [
        format!("/app/{space}/admin"),
        format!("/app/{space}/admin/widgets/members"),
    ] {
        assert_eq!(app.get(&uri).await.statut, StatusCode::FORBIDDEN, "{uri}");
    }
    assert_eq!(
        app.post_htmx(&format!("/app/{space}/admin/members/{autre}/remove"), "")
            .await
            .statut,
        StatusCode::FORBIDDEN,
        "un membre simple ne retire personne"
    );
    assert_eq!(
        app.post_htmx(
            &format!("/app/{space}/admin/members/{autre}/role"),
            "profile=SpaceAdmin"
        )
        .await
        .statut,
        StatusCode::FORBIDDEN,
        "ni ne promeut personne"
    );
}

#[sqlx::test]
async fn un_non_membre_n_atteint_aucun_endpoint_d_administration(pool: sqlx::PgPool) {
    let (_, _, membre) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    for uri in [
        format!("/app/{AUTRE_ESPACE}/admin"),
        format!("/app/{AUTRE_ESPACE}/admin/widgets/members"),
    ] {
        assert_eq!(app.get(&uri).await.statut, StatusCode::FORBIDDEN, "{uri}");
    }
    assert_eq!(
        app.post_htmx(
            &format!("/app/{AUTRE_ESPACE}/admin/members/{membre}/remove"),
            ""
        )
        .await
        .statut,
        StatusCode::FORBIDDEN
    );
}

// ── Les chemins nominaux ────────────────────────────────────────────────────

#[sqlx::test]
async fn promouvoir_un_membre_rend_sa_ligne_a_jour(pool: sqlx::PgPool) {
    let (space, _, membre) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/{membre}/role"),
            "profile=SpaceAdmin",
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("sam-row"), "la ligne est re-rendue");
    assert!(
        r.corps.contains(r#"value="SpaceAdmin" selected"#),
        "elle porte le nouveau rôle : {}",
        r.corps
    );
    assert_eq!(
        r.entete("hx-trigger")
            .map(|v| v.contains("memberRoleChanged")),
        Some(true)
    );
}

/// Le repost du rôle courant réussit — rien ne s'est passé, mais la réponse est
/// la même que tout autre succès : un 204 se lirait comme un trou dans un
/// journal, et forcerait le client à distinguer deux formes de réussite.
#[sqlx::test]
async fn reposter_le_role_courant_rend_la_ligne_comme_tout_succes(pool: sqlx::PgPool) {
    let (space, _, membre) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/{membre}/role"),
            "profile=SpaceUser",
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("sam-row"));
}

#[sqlx::test]
async fn retirer_un_membre_rend_un_corps_vide_et_declenche(pool: sqlx::PgPool) {
    let (space, _, membre) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(&format!("/app/{space}/admin/members/{membre}/remove"), "")
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.is_empty(), "la ligne disparaît : {}", r.corps);
    assert_eq!(
        r.entete("hx-trigger").map(|v| v.contains("memberRemoved")),
        Some(true)
    );
}

#[sqlx::test]
async fn un_coach_id_malforme_est_refuse(pool: sqlx::PgPool) {
    let (space, _, _) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/pas-un-ulid/remove"),
            "",
        )
        .await;

    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
}

// ── Le bouton de réinitialisation (carte 372) ───────────────────────────────

/// La destination est celle du BC d'authentification, injectée par l'hôte.
/// `spaces` ne la connaît pas : il rend la chaîne qu'on lui donne.
#[sqlx::test]
async fn la_ligne_porte_la_destination_de_reinitialisation(pool: sqlx::PgPool) {
    let (space, _, _) = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/members"))
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(
        r.corps.contains("/auth/password/request"),
        "le bouton doit poster vers l'endpoint injecté : {}",
        r.corps
    );
    assert!(
        r.corps.contains(r#"hx-vals='{"coach_name": "DevCoach"}'"#),
        "et transmettre le pseudo de la ligne : {}",
        r.corps
    );
}

/// L'endpoint d'`auth` rend **204**, sans corps ni redirection.
///
/// L'endpoint public, lui, rend `HX-Redirect` vers la page « consultez vos
/// emails » — ce qui ferait quitter l'application à un appelant qui l'invoque
/// depuis une ligne de tableau. C'est toute la raison d'être de cette variante.
#[sqlx::test]
async fn la_demande_de_reinitialisation_rend_204_sans_corps(pool: sqlx::PgPool) {
    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx("/auth/password/request", "coach_name=DevCoach")
        .await;

    assert_eq!(r.statut, StatusCode::NO_CONTENT);
    assert!(r.corps.is_empty());
    assert!(
        r.entete("hx-redirect").is_none(),
        "aucune redirection : l'appelant reste où il est"
    );
}

/// Un pseudo inconnu rend `204` comme un pseudo connu.
///
/// Distinguer les deux dirait à n'importe qui si un compte existe. C'est le
/// choix déjà fait par l'endpoint public, repris tel quel.
#[sqlx::test]
async fn un_pseudo_inconnu_ne_se_distingue_pas_d_un_pseudo_connu(pool: sqlx::PgPool) {
    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx("/auth/password/request", "coach_name=PersonneIci")
        .await;

    assert_eq!(r.statut, StatusCode::NO_CONTENT);
}
