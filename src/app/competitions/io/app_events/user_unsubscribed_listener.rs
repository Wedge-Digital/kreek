//! Un coach retiré d'un espace perd ses fonctions d'administration sur les
//! compétitions de cet espace.
//!
//! C'est la **seule** conséquence inter-BC du retrait d'un membre. Ses équipes
//! restent, la compétition se déroule, et la saisie des matchs n'est pas touchée
//! — elle n'est ouverte qu'aux administrateurs d'espace. Ce qui doit changer,
//! c'est son inscription dans `competitions_members`.
//!
//! Le paramètre s'appelle `app_event_bus`, et ce n'est pas cosmétique : c'est ce
//! nom qui signale à l'axe 5 de `check-arch` qu'il s'agit d'un listener
//! **cross-BC**, exempté de la règle de transaction unique. Un événement déjà
//! committé ailleurs ne peut pas partager sa transaction.

use crate::app::shared_kernel::identity::spaces_app_events::SpacesAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, pool: PgPool) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    if envelope.event_type != SpacesAppEvent::USER_UNSUBSCRIBED {
                        continue;
                    }
                    let Ok(event) =
                        serde_json::from_value::<SpacesAppEvent>(envelope.payload.clone())
                    else {
                        tracing::error!(
                            "competitions::user_unsubscribed_listener: payload invalide"
                        );
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    retirer_des_competitions(event, &pool)
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::user_unsubscribed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn retirer_des_competitions(event: SpacesAppEvent, pool: &PgPool) {
    let SpacesAppEvent::UserUnsubscribed {
        user_id, space_id, ..
    } = event
    else {
        return;
    };

    // Le sous-`SELECT` borne le retrait aux compétitions de **cet** espace. Sans
    // lui, un coach retiré d'un espace perdrait ses fonctions partout ailleurs.
    let resultat = sqlx::query(
        "DELETE FROM competitions_members
         WHERE  coach_id = $1
         AND    competition_id IN (SELECT id FROM competitions WHERE space_id = $2)",
    )
    .bind(user_id.to_string())
    .bind(space_id.to_string())
    .execute(pool)
    .await;

    match resultat {
        Ok(r) if r.rows_affected() > 0 => tracing::info!(
            coach = %user_id,
            space = %space_id,
            retirees = r.rows_affected(),
            "coach retiré de ses compétitions"
        ),
        // Le cas courant : le coach n'administrait aucune compétition. Rien à
        // dire, et surtout rien à signaler comme une anomalie.
        Ok(_) => {}
        Err(e) => tracing::error!(
            coach = %user_id,
            space = %space_id,
            "competitions::user_unsubscribed_listener: {e}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::{EventId, SpaceId, UserId};
    use crate::common::event_envelope::EventEnvelope;
    use crate::common::services::event_bus::event_bus::new_bus;

    async fn semer_competition(pool: &PgPool, id: &str, space_id: &str, nom: &str) {
        sqlx::query("INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, $3, '')")
            .bind(id)
            .bind(space_id)
            .bind(nom)
            .execute(pool)
            .await
            .expect("compétition semée");
    }

    async fn semer_administrateur(pool: &PgPool, competition_id: &str, coach_id: &str) {
        sqlx::query(
            "INSERT INTO competitions_members (competition_id, coach_id, competition_profile)
             VALUES ($1, $2, 'CompetitionAdmin')",
        )
        .bind(competition_id)
        .bind(coach_id)
        .execute(pool)
        .await
        .expect("administrateur semé");
    }

    async fn compte(pool: &PgPool, coach_id: &str, competition_id: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM competitions_members
             WHERE coach_id = $1 AND competition_id = $2",
        )
        .bind(coach_id)
        .bind(competition_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn evenement(user_id: UserId, space_id: SpaceId) -> SpacesAppEvent {
        SpacesAppEvent::UserUnsubscribed {
            event_id: EventId::new(),
            user_id,
            space_id,
        }
    }

    /// Le retrait est borné à **cet** espace.
    ///
    /// C'est la raison d'être de ce test : sans le sous-`SELECT` du `DELETE`, le
    /// coach perdrait ses fonctions dans toutes ses compétitions, partout. Et
    /// l'assertion sur l'autre espace est la moitié qui compte — celle sur
    /// l'espace visé passerait aussi avec un `DELETE` non borné.
    #[sqlx::test]
    async fn le_coach_est_retire_de_cet_espace_et_d_aucun_autre(pool: PgPool) {
        let coach = UserId::new();
        let ici = SpaceId::new();
        let ailleurs = SpaceId::new();
        let (c_ici, c_ailleurs) = ("01JCOMPICI0000000000000000", "01JCOMPAILLEURS0000000000");

        semer_competition(&pool, c_ici, &ici.to_string(), "Ligue d'ici").await;
        semer_competition(&pool, c_ailleurs, &ailleurs.to_string(), "Ligue d'ailleurs").await;
        semer_administrateur(&pool, c_ici, &coach.to_string()).await;
        semer_administrateur(&pool, c_ailleurs, &coach.to_string()).await;

        retirer_des_competitions(evenement(coach, ici), &pool).await;

        assert_eq!(
            compte(&pool, &coach.to_string(), c_ici).await,
            0,
            "le coach doit perdre ses fonctions dans l'espace qu'il quitte"
        );
        assert_eq!(
            compte(&pool, &coach.to_string(), c_ailleurs).await,
            1,
            "et les garder ailleurs — sans cette assertion, un DELETE non borné passerait"
        );
    }

    /// Les autres administrateurs de la même compétition ne bougent pas.
    #[sqlx::test]
    async fn les_autres_administrateurs_ne_sont_pas_touches(pool: PgPool) {
        let parti = UserId::new();
        let reste = UserId::new();
        let space = SpaceId::new();
        let c = "01JCOMPDEUX000000000000000";

        semer_competition(&pool, c, &space.to_string(), "Ligue partagée").await;
        semer_administrateur(&pool, c, &parti.to_string()).await;
        semer_administrateur(&pool, c, &reste.to_string()).await;

        retirer_des_competitions(evenement(parti, space), &pool).await;

        assert_eq!(compte(&pool, &parti.to_string(), c).await, 0);
        assert_eq!(compte(&pool, &reste.to_string(), c).await, 1);
    }

    /// Un coach qui n'administrait rien : aucune écriture, aucune erreur.
    #[sqlx::test]
    async fn un_coach_sans_competition_ne_provoque_rien(pool: PgPool) {
        let coach = UserId::new();
        let space = SpaceId::new();
        let c = "01JCOMPVIDE000000000000000";

        semer_competition(&pool, c, &space.to_string(), "Ligue sans lui").await;

        retirer_des_competitions(evenement(coach, space), &pool).await;

        assert_eq!(compte(&pool, &coach.to_string(), c).await, 0);
    }

    /// Le listener ne réagit qu'à son type d'événement.
    ///
    /// Le filtre est sur `envelope.event_type`, avant toute désérialisation : un
    /// autre app event du même BC ne doit rien déclencher.
    #[sqlx::test]
    async fn un_autre_type_d_evenement_ne_declenche_rien(pool: PgPool) {
        let coach = UserId::new();
        let space = SpaceId::new();
        let c = "01JCOMPAUTRE00000000000000";

        semer_competition(&pool, c, &space.to_string(), "Ligue intacte").await;
        semer_administrateur(&pool, c, &coach.to_string()).await;

        let bus = new_bus();
        init(&bus, pool.clone());

        let _ = bus.send(EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: coach.to_string(),
            event_type: SpacesAppEvent::USER_SUBSCRIBED.to_string(),
            tags: serde_json::json!({}),
            payload: serde_json::to_value(evenement(coach, space)).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        });
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }

        assert_eq!(
            compte(&pool, &coach.to_string(), c).await,
            1,
            "un app event d'un autre type ne doit rien retirer"
        );
    }
}
