//! Les URLs d'action des widgets d'inscription, au niveau handler.
//!
//! Ces boutons ont rendu un `400` muet sur la démo. Deux défauts distincts,
//! nés du même commit, révélés le jour où `space_scope_middleware` s'est mis à
//! exiger que `{space_id}` décode en ULID :
//!
//! - les handlers des deux widgets ne lisaient **jamais** le `{space_id}` de
//!   leur propre route et rendaient le gabarit avec `String::new()`, d'où
//!   `/app//team/…` ;
//! - `approve_all_enrollments()` ne prenait aucun paramètre et substituait le
//!   littéral `"_"`, d'où `/app/_/team/…`.
//!
//! **Le niveau handler est le seul qui prouve la correction.** Un test de
//! gabarit passerait le `space_id` lui-même : il n'aurait rien vu, puisque le
//! gabarit n'a jamais été fautif. Le défaut était dans ce que le handler lui
//! donnait.

use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const SEASON: &str = "01JBBBBBBBBBBBBBBBBBBBBBBB";
const COMPETITION: &str = "01JCCCCCCCCCCCCCCCCCCCCCCC";

async fn equipe(pool: &sqlx::PgPool, statut: &str) -> (String, String) {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let (space_id,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(pool)
            .await
            .expect("espace E2E semé");

    let team_id = crate::app::shared_kernel::identity::sulid::SUlid::new().to_string();
    sqlx::query(
        "INSERT INTO team_proj
            (team_id, space_id, team_name, coach_name, roster_name,
             competition_id, season_id, status, team_value, coach_id)
         VALUES ($1, $2, 'Orcacolas', 'Aegnor', 'Orc', $3, $4, $5, 0, 'c')",
    )
    .bind(&team_id)
    .bind(&space_id)
    .bind(COMPETITION)
    .bind(SEASON)
    .bind(statut)
    .execute(pool)
    .await
    .expect("équipe de test");

    (space_id, team_id)
}

fn widget_uri(chemin: &str, space_id: &str) -> String {
    format!("/app/{space_id}/team/widgets/{chemin}?competition_id={COMPETITION}&season_id={SEASON}")
}

/// Les deux formes que prenait le défaut. Aucune n'est rattrapable en aval :
/// `space_scope_middleware` refuse les deux par un `400` qui ne trace rien.
fn aucune_url_malformee(html: &str) {
    assert!(
        !html.contains("/app//"),
        "segment d'espace vide dans le fragment rendu"
    );
    assert!(
        !html.contains("/app/_/"),
        "bouche-trou « _ » dans le fragment rendu"
    );
    assert!(
        !html.contains("{space_id}"),
        "placeholder non substitué dans le fragment rendu"
    );
}

#[sqlx::test]
async fn le_widget_des_inscriptions_en_attente_porte_le_space_id(pool: sqlx::PgPool) {
    let (space_id, team_id) = equipe(&pool, "PendingEnrollment").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let vue = app.get(&widget_uri("pending", &space_id)).await;

    assert_eq!(vue.statut, StatusCode::OK);
    aucune_url_malformee(&vue.corps);
    assert!(
        vue.corps.contains(&format!(
            "/app/{space_id}/team/{team_id}/enrollment/approve"
        )),
        "le bouton « Valider » doit porter l'espace et l'équipe"
    );
    assert!(
        vue.corps
            .contains(&format!("/app/{space_id}/team/widgets/pending/approve-all")),
        "le bouton « Tout valider » doit porter l'espace"
    );
}

#[sqlx::test]
async fn le_widget_des_inscrites_porte_le_space_id(pool: sqlx::PgPool) {
    let (space_id, team_id) = equipe(&pool, "Enrolled").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let vue = app.get(&widget_uri("enrolled", &space_id)).await;

    assert_eq!(vue.statut, StatusCode::OK);
    aucune_url_malformee(&vue.corps);
    assert!(
        vue.corps.contains(&format!(
            "/app/{space_id}/team/{team_id}/enrollment/dismiss"
        )),
        "le bouton « Renvoyer » doit porter l'espace et l'équipe"
    );
}

/// La preuve que le symptôme observé sur la démo a disparu.
///
/// On n'assure **pas** un `200` : l'équipe n'est semée qu'en projection, donc
/// le use case ne la trouvera pas dans l'event store. Ce qui se joue ici est en
/// amont — la requête doit franchir `space_scope_middleware`, ce que ni
/// `/app//…` ni `/app/_/…` ne faisaient.
#[sqlx::test]
async fn les_actions_franchissent_le_controle_d_appartenance(pool: sqlx::PgPool) {
    let (space_id, team_id) = equipe(&pool, "PendingEnrollment").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    for chemin in [
        format!("/app/{space_id}/team/{team_id}/enrollment/approve"),
        format!("/app/{space_id}/team/{team_id}/enrollment/reject"),
        format!("/app/{space_id}/team/widgets/pending/approve-all?competition_id={COMPETITION}&season_id={SEASON}"),
    ] {
        let reponse = app.post_htmx(&chemin, "").await;
        assert_ne!(
            reponse.statut,
            StatusCode::BAD_REQUEST,
            "{chemin} est refusée par le contrôle d'appartenance"
        );
    }
}

/// Le défaut d'origine, reproduit sur les deux formes malformées. Il vaut
/// d'être figé : c'est ce qui rend les assertions ci-dessus autre chose qu'une
/// tautologie — sans lui, on ne saurait pas que le `400` vient bien de là.
#[sqlx::test]
async fn un_space_id_malforme_est_toujours_refuse(pool: sqlx::PgPool) {
    let (_, team_id) = equipe(&pool, "PendingEnrollment").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    for espace in ["", "_"] {
        let reponse = app
            .post_htmx(
                &format!("/app/{espace}/team/{team_id}/enrollment/approve"),
                "",
            )
            .await;
        assert!(
            reponse.statut == StatusCode::BAD_REQUEST || reponse.statut == StatusCode::NOT_FOUND,
            "un espace « {espace} » doit être refusé, reçu {}",
            reponse.statut
        );
    }
}
