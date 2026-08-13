//! Le cloisonnement des espaces, vérifié au niveau handler (cartes 311 et 315).
//!
//! Ces assertions rejouent les sondes `curl` de la carte 315, qui n'avaient pu
//! être faites qu'à la main faute de harnais. C'est le premier cas d'usage du
//! troisième étage de test, et le meilleur rapport valeur/effort du projet :
//! une matrice d'autorisation coûte ici des millisecondes, contre un scénario
//! de navigateur chacune.
//!
//! **Le test décisif est la *différence*** entre le même espace et un autre.
//! Une assertion qui ne vérifierait que le `404` croisé passerait tout aussi
//! bien si la ressource n'existait pas du tout — elle ne prouverait rien.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

const AUTRE_ESPACE: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";

/// Crée un joueur dans l'espace donné, par le vrai chemin d'écriture : le
/// repository et son event store, que `find_by_id` relit.
async fn creer_joueur(pool: &sqlx::PgPool, space_id: &SpaceId, team_id: &str) -> String {
    use crate::app::players::io::repository::player_repository::PgPlayerRepository;
    use crate::app::players::ports::IPlayerRepository;

    let player_id = PlayerId(crate::app::shared_kernel::identity::sulid::SUlid::new().to_string());
    let event = PlayerDomainEvent::PlayerCreated {
        player_id: player_id.clone(),
        team_id: TeamId(team_id.to_string()),
        space_id: space_id.clone(),
        position_name: PositionNameVo::try_new("Piétaille des Carrières".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("DEMO_GRANIT__PIETAILLE".to_string()).unwrap(),
        jersey: None,
        base_skills: vec![],
        starting_spp: Spp(0),
        starting_value: ValueKpo(50),
    };
    let repo = PgPlayerRepository::new(pool.clone());
    repo.append(&player_id, &TeamId(team_id.to_string()), &event, 1)
        .await
        .expect("création du joueur de test");
    player_id.0
}

async fn espace_e2e(pool: &sqlx::PgPool) -> String {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    let row: (String,) = sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
        .fetch_one(pool)
        .await
        .expect("espace E2E seedé");
    row.0
}

/// La preuve tient dans l'écart : `200` depuis l'espace du joueur, `404`
/// depuis un autre. Sans le garde, les deux rendraient `200`.
#[sqlx::test]
async fn un_joueur_n_est_lisible_que_depuis_son_espace(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let joueur = creer_joueur(&pool, &SpaceId::try_new(&space_id).unwrap(), "equipe-1").await;

    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let nominal = app
        .get(&format!("/app/{space_id}/players/{joueur}/debug"))
        .await;
    assert_eq!(nominal.statut, StatusCode::OK, "depuis son propre espace");

    let croise = app
        .get(&format!("/app/{AUTRE_ESPACE}/players/{joueur}/debug"))
        .await;
    assert_eq!(
        croise.statut,
        StatusCode::NOT_FOUND,
        "depuis un autre espace — et `404`, jamais `403` : rien ne doit \
         confirmer l'existence d'un joueur d'un autre espace"
    );
}

/// Le même écart sur un endpoint d'**écriture**. C'est celui-là qui comptait :
/// les cartes 307 et 308 avaient transformé un affichage indu en sept portes.
#[sqlx::test]
async fn un_joueur_n_est_modifiable_que_depuis_son_espace(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let joueur = creer_joueur(&pool, &SpaceId::try_new(&space_id).unwrap(), "equipe-1").await;

    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let croise = app
        .post_htmx(
            &format!("/app/{AUTRE_ESPACE}/players/{joueur}/customisation/spp/add"),
            "amount=5&expected_version=0",
        )
        .await;
    assert_eq!(croise.statut, StatusCode::NOT_FOUND);
}

/// Un identifiant d'espace mal formé est un `400`, pas un `404` : la requête
/// est malformée, il n'y a pas de ressource à chercher.
#[sqlx::test]
async fn un_identifiant_d_espace_invalide_est_un_400(pool: sqlx::PgPool) {
    let space_id = espace_e2e(&pool).await;
    let joueur = creer_joueur(&pool, &SpaceId::try_new(&space_id).unwrap(), "equipe-1").await;

    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;
    let reponse = app
        .get(&format!("/app/pas-un-ulid/players/{joueur}/debug"))
        .await;

    assert_eq!(reponse.statut, StatusCode::BAD_REQUEST);
}
