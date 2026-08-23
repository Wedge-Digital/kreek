//! L'idempotence des envois (R3), vérifiée sur une vraie base.
//!
//! Elle ne repose sur aucune ligne de Rust : c'est l'index unique qui la tient,
//! et `claim` ne fait que rapporter son verdict. Un test qui mockerait sqlx ne
//! vérifierait donc **rien du tout** — d'où `#[sqlx::test]`, qui monte une base
//! migrée par test.

use crate::app::competitions::domain::notification_delivery::{DeliveryKey, NotificationType};
use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::identity::ids::CoachId;

fn cle(round_id: Option<&str>) -> DeliveryKey {
    DeliveryKey {
        notification_type: NotificationType::RoundEve,
        season_id: SeasonId::try_new("01KZVCKDG19DXZHJA295WSJGMV").unwrap(),
        round_id: round_id.map(|r| MatchId::try_new(r).unwrap()),
        target_date: DateString::try_new("2026-09-01").unwrap(),
        coach_id: CoachId::try_new("01KZVCKDG19DXZHJA295WSJGMW").unwrap(),
    }
}

#[sqlx::test]
async fn deux_reservations_identiques_n_en_accordent_qu_une(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    let k = cle(Some("01KZVCKDG19DXZHJA295WSJGMX"));

    assert!(depot.claim(&k).await.unwrap(), "la première doit réserver");
    assert!(
        !depot.claim(&k).await.unwrap(),
        "la seconde doit trouver le créneau pris — c'est tout R3"
    );
}

/// **Le test que la carte met en avant.** Il échoue avec une contrainte
/// `UNIQUE` ordinaire, parce que PostgreSQL ne considère jamais deux `NULL`
/// comme égaux : les deux notifications de saison — celles qui n'ont pas de
/// journée — seraient alors dupliquées autant de fois qu'on relance le cron,
/// et seulement celles-là.
///
/// Le précédent passerait quand même. C'est ce qui rendrait le défaut invisible.
#[sqlx::test]
async fn deux_reservations_identiques_sans_journee_n_en_accordent_qu_une(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    let k = cle(None);

    assert!(depot.claim(&k).await.unwrap(), "la première doit réserver");
    assert!(
        !depot.claim(&k).await.unwrap(),
        "deux NULL doivent être traités comme égaux — c'est l'index sur COALESCE"
    );
}

#[sqlx::test]
async fn une_journee_differente_ouvre_un_nouveau_creneau(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);

    assert!(depot
        .claim(&cle(Some("01KZVCKDG19DXZHJA295WSJGMX")))
        .await
        .unwrap());
    assert!(
        depot
            .claim(&cle(Some("01KZVCKDG19DXZHJA295WSJGMY")))
            .await
            .unwrap(),
        "une autre journée est un autre envoi"
    );
}

#[sqlx::test]
async fn confirmer_renseigne_la_date_d_envoi(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool.clone());
    let k = cle(Some("01KZVCKDG19DXZHJA295WSJGMX"));

    depot.claim(&k).await.unwrap();
    // Réservée mais non confirmée : c'est l'état d'un échec constaté (R1).
    assert_eq!(sent_at(&pool).await, None);

    depot.confirm(&k).await.unwrap();
    assert!(
        sent_at(&pool).await.is_some(),
        "après confirmation, la ligne atteste de l'envoi"
    );
}

async fn sent_at(pool: &sqlx::PgPool) -> Option<time::OffsetDateTime> {
    sqlx::query_scalar("SELECT sent_at FROM competition_notification_deliveries")
        .fetch_one(pool)
        .await
        .unwrap()
}
