//! Le widget de création de compte fourni par le BC d'authentification.
//!
//! Ce que ces tests couvrent en priorité : **le contrat non typé**. Le nom de
//! l'événement DOM et ses deux clés franchissent une frontière de BC par le
//! navigateur, et ni le compilateur, ni `check-arch` ne les voient. Le harnais
//! peut au moins vérifier que l'en-tête est posé avec les bonnes clés ; seul un
//! test e2e peut vérifier que quelqu'un les écoute.

use crate::cli::seed_e2e::SIMPLE_COACH_NAME;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const WIDGET: &str = "/auth/widgets/coach-creation";

async fn app(pool: sqlx::PgPool) -> Harnais {
    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    Harnais::connecte_en_tant_que(pool, "DevCoach").await
}

#[sqlx::test]
async fn le_widget_rend_ses_deux_champs_et_aucun_selecteur_de_profil(pool: sqlx::PgPool) {
    let app = app(pool).await;

    let r = app.get(WIDGET).await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains(r#"name="coach_name""#));
    assert!(r.corps.contains(r#"name="email""#));
    assert!(
        !r.corps.contains("SpaceAdmin") && !r.corps.contains("SpaceUser"),
        "le rôle dans un espace est un concept de l'hôte : {}",
        r.corps
    );
}

#[sqlx::test]
async fn le_pre_remplissage_vient_de_l_appelant(pool: sqlx::PgPool) {
    let app = app(pool).await;

    let r = app
        .get(&format!("{WIDGET}?pseudo=NurgleFan&email=n%40bb.club"))
        .await;

    assert!(r.corps.contains(r#"value="NurgleFan""#));
    assert!(r.corps.contains(r#"value="n@bb.club""#));
}

/// Le contrat que rien d'autre ne vérifie.
#[sqlx::test]
async fn une_creation_reussie_pose_l_evenement_avec_ses_deux_cles(pool: sqlx::PgPool) {
    let app = app(pool).await;

    let r = app
        .post_htmx(WIDGET, "coach_name=NurgleFan&email=nurgle%40bb.club")
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    let entete = r.entete("hx-trigger").expect("l'en-tête doit être posé");
    assert!(entete.contains("accountCreated"), "{entete}");
    assert!(entete.contains("coach_id"), "{entete}");
    assert!(entete.contains(r#""name":"NurgleFan""#), "{entete}");
    assert!(
        r.corps.is_empty(),
        "le formulaire disparaît : il n'y a plus rien à saisir"
    );
}

/// Les erreurs restent chez ce BC : elles sont rendues dans son fragment, et
/// **aucun événement n'est posé**. C'est tout le bénéfice de la forme widget.
#[sqlx::test]
async fn un_pseudo_deja_pris_rend_l_erreur_dans_le_fragment(pool: sqlx::PgPool) {
    let app = app(pool).await;

    let r = app
        .post_htmx(
            WIDGET,
            &format!(
                "coach_name={}&email=libre%40bb.club",
                SIMPLE_COACH_NAME.replace(' ', "+")
            ),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("déjà pris"), "{}", r.corps);
    assert!(
        r.entete("hx-trigger").is_none(),
        "aucun événement ne doit être posé sur un échec"
    );
}

#[sqlx::test]
async fn une_adresse_invalide_rend_l_erreur_et_conserve_la_saisie(pool: sqlx::PgPool) {
    let app = app(pool).await;

    let r = app
        .post_htmx(WIDGET, "coach_name=NurgleFan&email=pas-une-adresse")
        .await;

    assert!(r.corps.contains("Adresse invalide"), "{}", r.corps);
    assert!(
        r.corps.contains(r#"value="NurgleFan""#),
        "la saisie est conservée : {}",
        r.corps
    );
    assert!(r.entete("hx-trigger").is_none());
}
