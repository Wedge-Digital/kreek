//! Le widget d'édition du logo d'équipe, du routeur à la projection.
//!
//! Ces tests frappent le routeur de production, avec sa garde : ils prouvent
//! que le POST du widget traverse le use case et écrit vraiment `team_proj`
//! (carte 509), et que le widget rend ce qu'il doit selon l'état du logo
//! (carte 510).

use super::fixtures::equipe_semee;
use crate::app::teams::routes::Routes;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const MEMBRE_SIMPLE: &str = crate::cli::seed_e2e::SIMPLE_COACH_NAME;
const COMMISSAIRE: &str = crate::cli::seed_e2e::DEV_COACH_NAME;
const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/v1/logo-equipe.jpg";

/// Le bouton de retrait tel que le template le rend. Le script du widget cite
/// aussi la classe `.team-logo-remove` : chercher la classe seule le trouverait
/// toujours.
const BOUTON_RETIRER: &str = r#"class="team-logo-remove""#;

fn formulaire(logo_url: &str) -> String {
    format!("logo_url={}", urlencoding::encode(logo_url))
}

async fn logo_en_projection(pool: &sqlx::PgPool, team_id: &str) -> Option<String> {
    let (logo,): (Option<String>,) =
        sqlx::query_as("SELECT logo_url FROM team_proj WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(pool)
            .await
            .expect("équipe projetée");
    logo
}

/// Le POST écrit l'événement **et** la projection : relire `team_proj` est ce
/// qui le prouve, la réponse seule pourrait venir d'un rendu sans écriture.
#[sqlx::test]
async fn un_logo_cloudinary_est_enregistre_dans_la_projection(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool.clone(), COMMISSAIRE).await;

    let r = admin
        .post_htmx(&Routes.team_logo_widget(&space, &team), &formulaire(LOGO))
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(
        logo_en_projection(&pool, &team).await.as_deref(),
        Some(LOGO)
    );
    assert!(
        r.corps.contains(LOGO),
        "le widget rendu montre le nouveau logo"
    );
}

/// Un champ vide vaut retrait : la projection repasse à NULL.
#[sqlx::test]
async fn retirer_le_logo_remet_la_projection_a_null(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool.clone(), COMMISSAIRE).await;
    let url = Routes.team_logo_widget(&space, &team);
    admin.post_htmx(&url, &formulaire(LOGO)).await;

    let r = admin.post_htmx(&url, &formulaire("")).await;

    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(logo_en_projection(&pool, &team).await, None);
}

/// Une URL hors Cloudinary est refusée **sous le champ**, en 200 — sinon htmx
/// n'échangerait rien — et rien n'est écrit.
#[sqlx::test]
async fn une_url_hors_cloudinary_est_refusee_sans_rien_ecrire(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool.clone(), COMMISSAIRE).await;

    let r = admin
        .post_htmx(
            &Routes.team_logo_widget(&space, &team),
            &formulaire("https://example.com/logo.png"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("Le logo doit être une image Cloudinary."));
    assert_eq!(logo_en_projection(&pool, &team).await, None);
}

/// Le bouton de retrait n'existe que s'il y a un logo à retirer.
#[sqlx::test]
async fn le_bouton_retirer_n_apparait_qu_avec_un_logo(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;
    let url = Routes.team_logo_widget(&space, &team);

    let sans_logo = admin.get(&url).await;
    admin.post_htmx(&url, &formulaire(LOGO)).await;
    let avec_logo = admin.get(&url).await;

    assert_eq!(sans_logo.statut, StatusCode::OK);
    assert!(!sans_logo.corps.contains(BOUTON_RETIRER), "rien à retirer");
    assert!(
        avec_logo.corps.contains(BOUTON_RETIRER),
        "un logo se retire"
    );
}

/// Le widget ne pose plus d'`id` à lui : deux widgets sur une même page ne
/// doivent pas se gêner (règle 6 des widgets). Les `id` de la macro d'upload
/// partagée (`logo_url`, `zone-logo_url`…) restent hors de son ressort.
#[sqlx::test]
async fn le_widget_ne_pose_aucun_identifiant_global(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let admin = Harnais::connecte_en_tant_que(pool, COMMISSAIRE).await;

    let r = admin.get(&Routes.team_logo_widget(&space, &team)).await;

    for id in [
        r#"id="team-logo-widget""#,
        r#"id="team-logo-form""#,
        r#"id="team-logo-close-btn""#,
        r#"id="team-logo-remove-btn""#,
        "getElementById('team-logo-form')",
    ] {
        assert!(!r.corps.contains(id), "« {id} » ne doit plus apparaître");
    }
}

/// La garde : un membre simple, ni propriétaire ni administrateur, est refusé.
#[sqlx::test]
async fn un_membre_simple_ne_peut_pas_changer_le_logo(pool: sqlx::PgPool) {
    let (space, team) = equipe_semee(&pool).await;
    let membre = Harnais::connecte_en_tant_que(pool.clone(), MEMBRE_SIMPLE).await;

    let r = membre
        .post_htmx(&Routes.team_logo_widget(&space, &team), &formulaire(LOGO))
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
    assert_eq!(logo_en_projection(&pool, &team).await, None);
}
