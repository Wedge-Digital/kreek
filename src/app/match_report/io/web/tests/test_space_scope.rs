//! Le cloisonnement des espaces pour `match_report` (carte 319).
//!
//! Ce BC n'avait pas été sondé pendant l'audit — une sonde d'écriture y aurait
//! laissé des traces difficiles à défaire. Son rang reposait sur la lecture du
//! code. **La sonde de lecture, faite au début de cette carte, a confirmé la
//! déduction** : `200` depuis un espace étranger, avec la page servie.
//!
//! La sémantique du refus est testée en carte 324 ; ici, seul le résolveur.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

/// Un rapport de match écrit directement en projection : le parcours réel
/// suppose une compétition, une journée, un pairing et deux équipes, dont rien
/// n'est nécessaire pour vérifier une appartenance.
async fn rapport_dans_l_espace_e2e(pool: &sqlx::PgPool) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E seedé");

    let match_report_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    // La projection est fortement contrainte : quinze colonnes non nulles. Les
    // valeurs sont sans importance ici — seul `space_id` est interrogé — mais
    // elles doivent exister.
    sqlx::query(
        "INSERT INTO match_report_proj
            (match_report_id, space_id, competition_id, season_id, round_id,
             home_team_id, away_team_id, created_by, origin, phase, version)
         VALUES ($1, $2, 'c', 's', 'r', 'h', 'a', 'test', 'test', 'draft', 1)",
    )
    .bind(&match_report_id)
    .bind(&space_id)
    .execute(pool)
    .await
    .expect("rapport de test");

    (space_id, match_report_id)
}

/// **L'écart est prouvé au niveau du résolveur, pas en HTTP** — et c'est une
/// contrainte de ce BC, pas un renoncement.
///
/// Les handlers de `match_report` chargent l'agrégat depuis l'event store ; un
/// rapport semé en projection seule leur rend `404` quel que soit l'espace. Le
/// cas nominal HTTP serait donc `404` lui aussi, et l'assertion ne
/// distinguerait pas le refus du middleware de l'absence d'agrégat — elle
/// passerait sans rien prouver.
///
/// Semer l'event store rendrait le test dépendant du format des événements, et
/// donc cassant à chaque évolution du domaine, pour un gain nul : ce qu'on
/// vérifie ici est l'appartenance, pas le rendu.
#[sqlx::test]
async fn le_resolveur_rend_l_espace_du_rapport_et_rien_pour_un_inconnu(pool: sqlx::PgPool) {
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::web::middleware::space_scope::ISpaceOwnership;

    let (space_id, match_report_id) = rapport_dans_l_espace_e2e(&pool).await;
    let resolveur =
        crate::infrastructure::match_report::space_ownership::MatchReportSpaceOwnership::new(
            std::sync::Arc::new(
                crate::app::match_report::io::repository::match_report_repository::MatchReportRepository::new(pool),
            ),
        );

    assert_eq!(resolveur.param(), "match_report_id");
    assert_eq!(
        resolveur.space_of(&match_report_id).await,
        Some(SpaceId::try_new(&space_id).unwrap()),
        "le rapport doit rendre l'espace où il a été créé"
    );
    assert_eq!(
        resolveur.space_of("01JZZZZZZZZZZZZZZZZZZZZZZZ").await,
        None,
        "un rapport inconnu ne rend aucun espace, donc un refus"
    );
}

/// Le refus vaut aussi en écriture — les rapports portent les données de jeu,
/// c'est le cas qui a valu à ce BC son rang de priorité.
#[sqlx::test]
async fn un_rapport_n_est_modifiable_que_depuis_son_espace(pool: sqlx::PgPool) {
    let (_, match_report_id) = rapport_dans_l_espace_e2e(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let croise = app
        .post_htmx(
            &format!("/app/{AUTRE_ESPACE}/match-report/{match_report_id}/step2"),
            "",
        )
        .await;

    assert_eq!(croise.statut, StatusCode::NOT_FOUND);
}
