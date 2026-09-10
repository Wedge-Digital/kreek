//! La route de réponse aux présences est **publique**, et ce test est ce qui l'y
//! maintient.
//!
//! # Pourquoi un test, et pas un commentaire
//!
//! Un commentaire n'a jamais arrêté personne, et l'erreur qu'il faut empêcher —
//! ranger `public_router()` dans `protected`, ou ajouter une route de présence à
//! côté des autres — **ne casserait aucun test existant**. `protected` porte
//! `route_layer(require_auth)` : la route redirigerait vers `/auth/login`, les
//! liens déjà partis dans les e-mails cesseraient de répondre, et on l'apprendrait
//! par un coach.
//!
//! Ces tests montent le **routeur de production** et demandent la route sans
//! cookie. Ils lisent ce que le routeur fait, pas ce qu'on croit lui avoir dit —
//! même patron que `test_cookie_de_session.rs`, et pour la même raison.

use crate::app::auth::routes::path::AUTH_LAYOUT;
use crate::web::test_harness::Harnais;

/// Un jeton de la bonne forme, mais qui ne désigne rien : la route doit répondre
/// une page, jamais rediriger.
const JETON: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

#[sqlx::test]
async fn la_route_de_reponse_repond_sans_session(pool: sqlx::PgPool) {
    let harnais = Harnais::sans_session(pool).await;

    for chemin in [
        format!("/presence/{JETON}/oui"),
        format!("/presence/{JETON}/non"),
    ] {
        let reponse = harnais.get_anonyme(&chemin).await;

        assert_ne!(
            reponse.statut,
            axum::http::StatusCode::SEE_OTHER,
            "{chemin} redirige : la route est passée sous `require_auth`, et tous \
             les liens déjà envoyés par e-mail sont morts"
        );
        assert!(
            reponse.entete("location").unwrap_or_default() != AUTH_LAYOUT,
            "{chemin} renvoie vers la connexion — cf. le commentaire de \
             `public_router()`"
        );
        assert_eq!(
            reponse.statut,
            axum::http::StatusCode::OK,
            "{chemin} doit rendre une page, fût-elle « lien inconnu »"
        );
    }
}

/// R26 — un jeton qui ne désigne rien rend la page « lien inconnu », et cette page
/// **ne dit pas que le lien a expiré** : ce serait affirmer qu'il a existé.
#[sqlx::test]
async fn un_jeton_inconnu_rend_une_page_qui_ne_revele_rien(pool: sqlx::PgPool) {
    let harnais = Harnais::sans_session(pool).await;

    let reponse = harnais.get_anonyme(&format!("/presence/{JETON}/oui")).await;

    assert_eq!(reponse.statut, axum::http::StatusCode::OK);
    assert!(
        reponse.corps.contains("ne mène à rien"),
        "la page « lien inconnu » doit être rendue"
    );
    assert!(
        !reponse.corps.contains(JETON),
        "le jeton ne doit pas reparaître dans la page"
    );
    // **Des phrases, pas des fragments.** Un premier jet cherchait « clos », qui
    // matche `onclose` dans le script de rechargement à chaud injecté en debug :
    // le test échouait sur une page juste. Même défaut qu'un `contains("01")` sur
    // un nom d'équipe horodaté.
    for revelateur in [
        "Ce sondage est clos",
        "C'est noté",
        "Ton équipe",
        "Compétition",
    ] {
        assert!(
            !reponse.corps.contains(revelateur),
            "« {revelateur} » dans la page révélerait que le lien a existé (R26)"
        );
    }
}

/// **La garantie de R26 est d'abord structurelle**, et ce test la constate :
/// `PresenceUnknownTemplate` ne porte qu'un champ, le chemin du bundle CSS. Un
/// gabarit qui ne reçoit ni équipe, ni journée, ni compétition ne peut rien en
/// laisser filtrer — quelle que soit la prose qu'on y écrira demain.
#[test]
fn la_page_lien_inconnu_ne_recoit_aucune_donnee() {
    use crate::app::competitions::io::web::public::presence_response::PresenceUnknownTemplate;
    use askama::Template;

    let page = PresenceUnknownTemplate {
        css: "/css/app.css",
    }
    .render()
    .expect("rendu");

    assert!(page.contains("ne mène à rien"));
    // Le seul champ du gabarit est le CSS : s'il en gagnait un second, cette
    // construction ne compilerait plus, et c'est le vrai verrou.
    assert!(page.contains("/css/app.css"));
}

/// **Le troisième verbe est refusé par le routeur**, pas par le code : deux
/// chemins littéraux valent mieux qu'un verbe extrait puis rejeté dans un `match`
/// — la contrainte est portée par la déclaration, il n'y a rien à oublier.
#[sqlx::test]
async fn un_troisieme_verbe_n_existe_pas(pool: sqlx::PgPool) {
    let harnais = Harnais::sans_session(pool).await;

    let reponse = harnais
        .get_anonyme(&format!("/presence/{JETON}/peut-etre"))
        .await;

    assert_eq!(reponse.statut, axum::http::StatusCode::NOT_FOUND);
}
