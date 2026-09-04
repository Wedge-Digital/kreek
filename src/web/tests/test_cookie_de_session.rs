//! Ce que le cookie de session emporte (carte 490).
//!
//! Plusieurs coachs se plaignaient d'être déconnectés trop souvent. Le cookie
//! n'avait **ni `Max-Age` ni `Expires`** : `SessionManagerLayer::new(...)` sans
//! `with_expiry` laisse `expiry: None`, que `tower-sessions` traduit par un
//! cookie de session — le navigateur le jette à sa fermeture. Le serveur, lui,
//! gardait la session deux semaines.
//!
//! Ces tests lisent l'en-tête **réellement émis** par le routeur de production,
//! pas la configuration qui est censée le produire.

use crate::web::test_harness::Harnais;

/// Le compte semé par `seed_e2e`, celui qu'emploient les autres tests du harnais.
const COACH: &str = "DevCoach";

/// La base de `sqlx::test` est vierge : sans semis, la connexion échoue et les
/// trois tests rendent « connexion refusée » au lieu de dire quoi que ce soit du
/// cookie.
async fn semer(pool: &sqlx::PgPool) {
    // `execute` prend une référence.
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
}

#[sqlx::test]
async fn le_cookie_de_session_dure_au_dela_de_la_fermeture_du_navigateur(pool: sqlx::PgPool) {
    semer(&pool).await;
    let harnais = Harnais::connecte_en_tant_que(pool, COACH).await;
    let brut = harnais.set_cookie_brut().to_lowercase();

    assert!(
        brut.contains("max-age=") || brut.contains("expires="),
        "le cookie n'a pas de durée : le navigateur le jettera à sa fermeture — {brut}"
    );

    // 30 jours = 2 592 000 secondes. On vérifie l'ordre de grandeur plutôt que la
    // valeur exacte : `OnInactivity` la recalcule à chaque requête, et un test
    // qui exige la seconde près se casse sur une machine lente.
    let secondes: i64 = brut
        .split("max-age=")
        .nth(1)
        .and_then(|s| s.split(';').next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    assert!(
        secondes > 20 * 86_400,
        "durée de {secondes} s : trop courte pour une ligue où deux matchs sont \
         séparés d'une à deux semaines"
    );
}

#[sqlx::test]
async fn le_cookie_survit_a_une_arrivee_depuis_un_lien_exterieur(pool: sqlx::PgPool) {
    semer(&pool).await;
    let harnais = Harnais::connecte_en_tant_que(pool, COACH).await;
    let brut = harnais.set_cookie_brut().to_lowercase();

    assert!(
        brut.contains("samesite=lax"),
        "un cookie `Strict` n'est pas envoyé quand on arrive par un lien externe — \
         le coach atterrit déconnecté sans avoir rien fait : {brut}"
    );
}

/// **La contre-épreuve.** Assouplir `SameSite` ne doit pas assouplir le reste :
/// `HttpOnly` tient le cookie hors de portée du JavaScript, `Secure` hors du
/// HTTP en clair. Sans ce test, un futur `with_secure(false)` posé pour déboguer
/// en local partirait en production sans que rien ne proteste.
#[sqlx::test]
async fn le_cookie_reste_http_only_et_secure(pool: sqlx::PgPool) {
    semer(&pool).await;
    let harnais = Harnais::connecte_en_tant_que(pool, COACH).await;
    let brut = harnais.set_cookie_brut().to_lowercase();

    assert!(
        brut.contains("httponly"),
        "cookie lisible par le JS : {brut}"
    );
    assert!(
        brut.contains("secure"),
        "cookie transmissible en clair : {brut}"
    );
}
