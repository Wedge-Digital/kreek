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

// ── Ce que le journal sait dire d'une journée — carte 544 ────────────────────

const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";
const JOURNEE: &str = "01KZVCKDG19DXZHJA295WSJGMY";

fn envoi(type_: NotificationType, jour: &str, coach: &str) -> DeliveryKey {
    DeliveryKey {
        notification_type: type_,
        season_id: SeasonId::try_new(SAISON).unwrap(),
        round_id: Some(MatchId::try_new(JOURNEE).unwrap()),
        target_date: DateString::try_new(jour).unwrap(),
        coach_id: CoachId::try_new(coach).unwrap(),
    }
}

const ALICE: &str = "01KZVCKDG19DXZHJA295WSJG01";
const BOB: &str = "01KZVCKDG19DXZHJA295WSJG02";

/// **Le cas qui donne son sens à la lecture.** Une ligne réservée dont l'envoi a
/// échoué reste à `sent_at NULL` : c'est l'échec constaté que R20 veut
/// journaliser, et c'est la différence entre `reserves` et `attestes` qui le dit.
#[sqlx::test]
async fn une_reservation_non_confirmee_compte_comme_un_echec(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    let (a, b) = (
        envoi(NotificationType::PresenceSurvey, "2026-10-10", ALICE),
        envoi(NotificationType::PresenceSurvey, "2026-10-10", BOB),
    );
    depot.claim(&a).await.unwrap();
    depot.claim(&b).await.unwrap();
    // Seul le premier est attesté : le second a échoué à l'envoi.
    depot.confirm(&a).await.unwrap();

    let comptes = depot.count_by_round(SAISON, JOURNEE).await.unwrap();

    assert_eq!(comptes.len(), 1, "un seul envoi, donc une seule ligne");
    assert_eq!(comptes[0].reserves, 2);
    assert_eq!(comptes[0].attestes, 1);
}

/// La relance porte le **jour de l'envoi**, donc mardi et jeudi donnent deux
/// lignes. Un groupement par type seul les aurait additionnées en un nombre que
/// personne ne sait lire — et l'écran veut la plus récente.
#[sqlx::test]
async fn deux_relances_de_jours_differents_donnent_deux_lignes(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    for jour in ["2026-10-06", "2026-10-08"] {
        let k = envoi(NotificationType::PresenceReminder, jour, ALICE);
        depot.claim(&k).await.unwrap();
        depot.confirm(&k).await.unwrap();
    }

    let comptes = depot.count_by_round(SAISON, JOURNEE).await.unwrap();

    assert_eq!(comptes.len(), 2);
    // Ordonnées par date : la dernière est la plus récente, celle que l'écran
    // montre.
    assert_eq!(comptes[1].target_date, "2026-10-08");
}

/// Les deux types cohabitent sans se mélanger — c'est ce que la seconde variante
/// de `NotificationType` achète, et l'écran les affiche séparément.
#[sqlx::test]
async fn l_ouverture_et_la_relance_se_comptent_a_part(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    for k in [
        envoi(NotificationType::PresenceSurvey, "2026-10-10", ALICE),
        envoi(NotificationType::PresenceSurvey, "2026-10-10", BOB),
        envoi(NotificationType::PresenceReminder, "2026-10-06", BOB),
    ] {
        depot.claim(&k).await.unwrap();
        depot.confirm(&k).await.unwrap();
    }

    let comptes = depot.count_by_round(SAISON, JOURNEE).await.unwrap();

    let ouverture: Vec<_> = comptes
        .iter()
        .filter(|c| c.notification_type == "presence_survey")
        .collect();
    let relance: Vec<_> = comptes
        .iter()
        .filter(|c| c.notification_type == "presence_reminder")
        .collect();
    assert_eq!(ouverture.len(), 1);
    assert_eq!(ouverture[0].attestes, 2);
    assert_eq!(relance.len(), 1);
    assert_eq!(relance[0].attestes, 1);
}

/// Rien n'est parti : la ligne d'expédition ne s'affiche pas du tout.
#[sqlx::test]
async fn une_journee_sans_envoi_ne_compte_rien(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);

    let comptes = depot.count_by_round(SAISON, JOURNEE).await.unwrap();

    assert!(comptes.is_empty());
}

/// Le journal d'une **autre** journée ne déborde pas sur celle-ci. Sans le filtre
/// sur `round_id`, l'écran d'une journée annoncerait les envois de toute la saison.
#[sqlx::test]
async fn les_envois_d_une_autre_journee_ne_sont_pas_comptes(pool: sqlx::PgPool) {
    let depot = NotificationDeliveryRepository::new(pool);
    let ici = envoi(NotificationType::PresenceSurvey, "2026-10-10", ALICE);
    let mut ailleurs = ici.clone();
    ailleurs.round_id = Some(MatchId::try_new("01KZVCKDG19DXZHJA295WSJGMZ").unwrap());
    depot.claim(&ici).await.unwrap();
    depot.claim(&ailleurs).await.unwrap();

    let comptes = depot.count_by_round(SAISON, JOURNEE).await.unwrap();

    assert_eq!(comptes.len(), 1);
    assert_eq!(comptes[0].reserves, 1, "une seule des deux réservations");
}
