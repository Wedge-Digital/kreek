//! Le cloisonnement des espaces pour `teams` (carte 320).
//!
//! L'audit n'avait prouvé qu'une fuite en **lecture** : sa sonde d'écriture
//! avait rendu `422`, mais sur une équipe en phase `ReadyToPlay` — donc
//! vraisemblablement le garde de phase, pas l'autorisation.
//!
//! **La sonde refaite sur une équipe en phase de recrutement a levé le doute**,
//! et le résultat est pire que la lecture :
//!
//! ```text
//! POST /app/<espace étranger>/teams/<équipe>/recruitment/players/add
//! → 200, ligne écrite — et le panier estampillé de l'espace de l'ATTAQUANT
//! ```
//!
//! Un admin d'un espace quelconque recrutait donc dans l'équipe d'un autre,
//! touchant effectif et trésorerie.
//!
//! La sémantique du refus est testée en carte 324 ; ici, seul le résolveur.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

async fn equipe_dans_l_espace_e2e(pool: &sqlx::PgPool) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E seedé");

    let team_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    sqlx::query(
        "INSERT INTO team_proj
            (team_id, space_id, team_name, coach_name, roster_name, status, team_value, coach_id)
         VALUES ($1, $2, 'Équipe de test', 'DevCoach', 'Granitiers', 'Enrolled', 0, 'c')",
    )
    .bind(&team_id)
    .bind(&space_id)
    .execute(pool)
    .await
    .expect("équipe de test");

    (space_id, team_id)
}

/// **L'écart est prouvé au niveau du résolveur** — deuxième BC dans ce cas
/// après `match_report`, et pour la même raison : `team_detail` charge
/// l'agrégat depuis l'event store, donc une équipe semée en projection seule
/// rend `404` quel que soit l'espace. L'assertion nominale en HTTP serait
/// `404 != 404`, verte sans rien prouver.
///
/// Semer l'event store lierait le test au format des événements — cassant à
/// chaque évolution du domaine — pour vérifier une appartenance qui n'en dépend
/// pas.
///
/// L'écart en HTTP a été vérifié à la main, sur les données réelles du serveur
/// de développement, et il est consigné dans la carte.
#[sqlx::test]
async fn le_resolveur_rend_l_espace_de_l_equipe_et_rien_pour_une_inconnue(pool: sqlx::PgPool) {
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::web::middleware::space_scope::ISpaceOwnership;

    let (space_id, team_id) = equipe_dans_l_espace_e2e(&pool).await;
    let resolveur = crate::infrastructure::teams::space_ownership::TeamSpaceOwnership::new(
        std::sync::Arc::new(
            crate::app::teams::io::repository::team_repository::TeamRepository::new(
                pool.clone(),
                crate::common::services::event_bus::event_bus::new_bus(),
            ),
        ),
        std::sync::Arc::new(
            crate::app::team_creation::io::team_creation_repository::TeamDraftRepository::new(pool),
        ),
    );

    assert_eq!(resolveur.param(), "team_id");
    assert_eq!(
        resolveur.space_of(&team_id).await,
        Some(SpaceId::try_new(&space_id).unwrap()),
        "l'équipe doit rendre l'espace où elle a été créée"
    );
    assert_eq!(
        resolveur.space_of("01JZZZZZZZZZZZZZZZZZZZZZZZ").await,
        None,
        "une équipe inconnue ne rend aucun espace, donc un refus"
    );
}

/// Un **brouillon** n'est pas encore une équipe : il vit dans `team_drafts` et
/// n'apparaît dans `team_proj` qu'à sa soumission.
///
/// Ce test existe parce que le résolveur de la carte 320, qui ne lisait que la
/// projection, a rendu `404` sur tous les brouillons — cassant la création
/// d'équipe pour 47 d'entre eux, sans qu'aucun test unitaire ne bronche. Seule
/// la suite e2e l'aurait vu, et elle n'avait pas été lancée.
#[sqlx::test]
async fn un_brouillon_non_soumis_reste_atteignable(pool: sqlx::PgPool) {
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::web::middleware::space_scope::ISpaceOwnership;

    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(&pool)
            .await
            .expect("espace E2E seedé");

    // Un brouillon, et lui seul : rien dans `team_proj`.
    let draft_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    sqlx::query(
        "INSERT INTO team_drafts
            (id, space_id, competition_id, season_id, name, coach_id, coach_name,
             creation_rules, status)
         VALUES ($1, $2, 'c', 's', 'Brouillon', 'coach', 'DevCoach', '{}', 'draft')",
    )
    .bind(&draft_id)
    .bind(&space_id)
    .execute(&pool)
    .await
    .expect("brouillon de test");

    let resolveur = crate::infrastructure::teams::space_ownership::TeamSpaceOwnership::new(
        std::sync::Arc::new(
            crate::app::teams::io::repository::team_repository::TeamRepository::new(
                pool.clone(),
                crate::common::services::event_bus::event_bus::new_bus(),
            ),
        ),
        std::sync::Arc::new(
            crate::app::team_creation::io::team_creation_repository::TeamDraftRepository::new(pool),
        ),
    );

    assert_eq!(
        resolveur.space_of(&draft_id).await,
        Some(SpaceId::try_new(&space_id).unwrap()),
        "un brouillon absent de team_proj doit rendre son espace, sans quoi la \
         création d'équipe est cassée"
    );
}

/// Le cas que la sonde a prouvé exploitable, et le plus lourd des trois : il
/// touchait l'effectif et la trésorerie.
#[sqlx::test]
async fn le_recrutement_croise_est_refuse(pool: sqlx::PgPool) {
    let (_, team_id) = equipe_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let croise = app
        .post_htmx(
            &format!("/app/{AUTRE_ESPACE}/teams/{team_id}/recruitment/players/add"),
            "roster_line_id=DEMO_GRANIT__PIETAILLE&version=0",
        )
        .await;

    assert_eq!(croise.statut, StatusCode::NOT_FOUND);
}
