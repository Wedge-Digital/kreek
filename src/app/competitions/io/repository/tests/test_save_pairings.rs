//! L'atomicité de l'écriture d'une journée, sur une vraie base.
//!
//! Ce que `save_pairings` garantit ne repose sur aucune ligne de Rust
//! vérifiable en mémoire : c'est la transaction, le verrou de ligne et la
//! relecture sous ce verrou. Un test qui mockerait sqlx ne vérifierait rien —
//! d'où `#[sqlx::test]`, qui monte une base migrée par test.
//!
//! **Ce que ce fichier ne prouve pas** : l'exclusion mutuelle elle-même, qui
//! est le travail de PostgreSQL. Deux transactions réellement concurrentes
//! demanderaient deux connexions et un ordonnancement contrôlé, dont la
//! fragilité coûterait plus qu'elle ne rapporte. Ce qui est éprouvé ici est la
//! **relecture** — la partie qui nous appartient, et sans laquelle le verrou ne
//! serait qu'un ralentisseur : il sérialiserait deux écritures fautives au lieu
//! de les paralléliser.

use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError, NewPairingProjection,
};
use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";
const JOURNEE: &str = "01KZVCKDG19DXZHJA295WSJGMX";

async fn poser_la_journee(pool: &sqlx::PgPool) {
    sqlx::query(
        "INSERT INTO competition_match_days (id, season_id, name, day_type, position)
         VALUES ($1, $2, 'Journée 1', 'fixed_date', 0)",
    )
    .bind(JOURNEE)
    .bind(SAISON)
    .execute(pool)
    .await
    .expect("journée de test");
}

fn appariement() -> (Pairing, NewPairingProjection) {
    (
        Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
        },
        NewPairingProjection {
            season_id: SAISON.to_string(),
            round_name: "Journée 1".to_string(),
            round_position: 0,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "fixed_date".to_string(),
            home_team_name: "Les Rats d'Égouts".to_string(),
            home_roster_name: "Skavens".to_string(),
            home_coach_name: "Lepandawan".to_string(),
            home_logo_url: None,
            away_team_name: "Les Crocs du Chaos".to_string(),
            away_roster_name: "Chaos".to_string(),
            away_coach_name: "Bagouze".to_string(),
            away_logo_url: None,
        },
    )
}

async fn compter(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM competition_match_day_pairings WHERE match_day_id = $1",
    )
    .bind(JOURNEE)
    .fetch_one(pool)
    .await
    .expect("comptage")
}

#[sqlx::test]
async fn les_appariements_d_une_journee_s_ecrivent_ensemble(pool: sqlx::PgPool) {
    poser_la_journee(&pool).await;
    let depot = MatchDayRepository::new(pool.clone());
    let paires = vec![appariement(), appariement(), appariement()];

    depot
        .save_pairings(JOURNEE, &paires)
        .await
        .expect("écriture");

    assert_eq!(compter(&pool).await, 3);
    let projections: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM competition_match_display_proj WHERE round_id = $1",
    )
    .bind(JOURNEE)
    .fetch_one(&pool)
    .await
    .expect("comptage");
    assert_eq!(projections, 3, "une projection par appariement");
}

/// **Le test que la carte met en avant.** Sans la relecture sous verrou, la
/// seconde génération écrirait ses appariements par-dessus les premiers : le
/// use case a lu « journée vide » avant d'ouvrir sa transaction, et rien ne le
/// détromperait.
#[sqlx::test]
async fn une_seconde_generation_est_refusee(pool: sqlx::PgPool) {
    poser_la_journee(&pool).await;
    let depot = MatchDayRepository::new(pool.clone());

    depot
        .save_pairings(JOURNEE, &[appariement()])
        .await
        .expect("la première écrit");

    let refus = depot.save_pairings(JOURNEE, &[appariement()]).await;

    assert!(
        matches!(refus, Err(MatchDayRepositoryError::PairingsAlreadyExist)),
        "la seconde doit être refusée, obtenu : {refus:?}"
    );
    assert_eq!(compter(&pool).await, 1, "rien n'a été ajouté");
}

/// Une journée vide reste vide quand il n'y a rien à écrire — le cas d'une
/// poule sans effectif suffisant, qui ne doit pas verrouiller pour rien.
#[sqlx::test]
async fn ecrire_zero_appariement_ne_verrouille_rien(pool: sqlx::PgPool) {
    poser_la_journee(&pool).await;
    let depot = MatchDayRepository::new(pool.clone());

    depot
        .save_pairings(JOURNEE, &[])
        .await
        .expect("aucune écriture");

    assert_eq!(compter(&pool).await, 0);
}
