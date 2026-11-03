//! La session survit au serveur (carte 491).
//!
//! Les sessions vivaient dans un `DashMap` en mémoire, instancié par
//! `build_router` : chaque redéploiement déconnectait tout le monde en même
//! temps. Elles vivent désormais dans la table `sessions`, rangées sous
//! l'empreinte de leur identifiant.
//!
//! Le premier test est celui que la carte désigne comme le seul qui compte : il
//! échouait par construction avec le magasin en mémoire.

use std::str::FromStr;

use axum::http::StatusCode;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use crate::app::auth::routes::path as auth_path;
use crate::common::session_store::{hashed, purge_expired_sessions, raw_postgres_store};
use crate::web::routes::path::APP_LAYOUT;
use crate::web::test_harness::Harnais;

const COACH: &str = "DevCoach";

async fn seed(pool: &sqlx::PgPool) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
}

async fn stored_ids(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(pool)
        .await
        .expect("lecture de la table sessions")
}

/// Connecté, `/app` renvoie vers l'accueil de l'espace du coach ; anonyme, vers
/// la page de connexion. La destination distingue les deux.
fn assert_authenticated(statut: StatusCode, location: Option<&str>, contexte: &str) {
    assert_eq!(statut, StatusCode::SEE_OTHER, "{contexte}");
    assert!(
        location.is_some_and(|l| l.starts_with("/app/") && l.ends_with("/home")),
        "{contexte} — location : {location:?}"
    );
}

#[sqlx::test]
async fn a_session_survives_a_server_restart(pool: sqlx::PgPool) {
    seed(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool.clone(), COACH).await;
    let avant = app.get(APP_LAYOUT).await;
    assert_authenticated(
        avant.statut,
        avant.entete("location"),
        "avant le redémarrage",
    );

    let app = app.after_restart(pool).await;
    let apres = app.get(APP_LAYOUT).await;
    assert_authenticated(
        apres.statut,
        apres.entete("location"),
        "après le redémarrage, le cookie ne retrouve plus sa session",
    );
}

/// La table ne contient pas la valeur du cookie, mais son empreinte.
#[sqlx::test]
async fn the_table_stores_the_hash_not_the_cookie(pool: sqlx::PgPool) {
    seed(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool.clone(), COACH).await;
    let cookie = app.session_cookie_value().to_string();
    let attendu = hashed(&Id::from_str(&cookie).expect("cookie décodable")).to_string();

    let lignes = stored_ids(&pool).await;
    assert_ne!(
        lignes,
        vec![cookie],
        "la table contient la valeur du cookie"
    );
    assert_eq!(lignes, vec![attendu]);
}

/// **La contre-épreuve de l'empreinte.** Une ligne lue dans la table — un dump,
/// une sauvegarde — présentée comme cookie ne connecte personne.
#[sqlx::test]
async fn a_stolen_row_is_not_a_valid_cookie(pool: sqlx::PgPool) {
    seed(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool.clone(), COACH).await;
    let ligne = stored_ids(&pool).await.remove(0);

    let voleur = app.with_session_cookie(&ligne);
    let reponse = voleur.get(APP_LAYOUT).await;
    assert_eq!(reponse.entete("location"), Some(auth_path::LOGIN));
}

#[sqlx::test]
async fn logout_deletes_the_row(pool: sqlx::PgPool) {
    seed(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool.clone(), COACH).await;
    assert_eq!(stored_ids(&pool).await.len(), 1);

    app.get(auth_path::LOGOUT).await;
    assert!(
        stored_ids(&pool).await.is_empty(),
        "la déconnexion laisse sa ligne"
    );
}

fn record_expiring_in(jours: i64) -> Record {
    Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(jours),
    }
}

#[sqlx::test]
async fn purge_removes_expired_sessions_and_keeps_the_others(pool: sqlx::PgPool) {
    let store = raw_postgres_store(pool.clone());
    let expiree = record_expiring_in(-1);
    let valide = record_expiring_in(1);
    store.save(&expiree).await.unwrap();
    store.save(&valide).await.unwrap();

    purge_expired_sessions(&store).await;
    assert_eq!(stored_ids(&pool).await, vec![valide.id.to_string()]);
}
