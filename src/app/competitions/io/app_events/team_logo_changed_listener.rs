//! Une équipe a changé de logo, ou l'a retiré : la projection d'affichage des
//! matchs suit.
//!
//! `competitions` copie le logo de chaque équipe dans
//! `competition_match_display_proj` au moment de l'appariement. Sans ce
//! listener, le calendrier, les résultats et la liste des matchs d'une équipe
//! garderaient l'ancien logo — ou un logo que le coach a retiré.
//!
//! Le paramètre s'appelle `app_event_bus` : c'est ce nom qui signale à l'axe 5
//! de `check-arch` un listener **cross-BC**, exempté de la règle de transaction
//! unique. L'événement a été committé dans `teams`, il ne peut pas partager sa
//! transaction.

use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, pool: PgPool) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => traiter(envelope, &pool).await,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::team_logo_changed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn traiter(envelope: EventEnvelope, pool: &PgPool) {
    if envelope.event_type != TeamsAppEvent::LOGO_CHANGED {
        return;
    }
    let Ok(event) = serde_json::from_value::<TeamsAppEvent>(envelope.payload.clone()) else {
        tracing::error!("competitions::team_logo_changed_listener: payload invalide");
        return;
    };
    let span = tracing::info_span!(
        "app_event",
        event = %envelope.event_type,
        event_id = %envelope.event_id
    );
    mettre_a_jour_le_logo(event, pool).instrument(span).await;
}

async fn mettre_a_jour_le_logo(event: TeamsAppEvent, pool: &PgPool) {
    let TeamsAppEvent::LogoChanged {
        team_id, logo_url, ..
    } = event
    else {
        return;
    };

    // Une équipe joue à domicile ou à l'extérieur selon le match : chaque côté
    // n'est réécrit que s'il est le sien. Les valeurs sont absolues, donc
    // rejouer l'événement ne change rien.
    let resultat = sqlx::query(
        "UPDATE competition_match_display_proj
         SET    home_logo_url = CASE WHEN home_team_id = $1 THEN $2 ELSE home_logo_url END,
                away_logo_url = CASE WHEN away_team_id = $1 THEN $2 ELSE away_logo_url END
         WHERE  home_team_id = $1 OR away_team_id = $1",
    )
    .bind(team_id.to_string())
    .bind(logo_url)
    .execute(pool)
    .await;

    match resultat {
        Ok(r) => tracing::info!(
            team = %team_id,
            matchs = r.rows_affected(),
            "logo d'équipe reporté sur les matchs"
        ),
        Err(e) => tracing::error!(team = %team_id, "competitions::team_logo_changed_listener: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{EventId, SpaceId};
    use crate::common::services::event_bus::event_bus::new_bus;

    const EQUIPE: &str = "01J00000000000000000000001";
    const ADVERSAIRE: &str = "01J00000000000000000000002";
    const ANCIEN: &str = "https://res.cloudinary.com/demo/image/upload/v1/ancien.jpg";
    const NOUVEAU: &str = "https://res.cloudinary.com/demo/image/upload/v1/nouveau.jpg";
    const AUTRE: &str = "https://res.cloudinary.com/demo/image/upload/v1/autre.jpg";

    async fn semer_match(pool: &PgPool, pairing_id: &str, domicile: &str, exterieur: &str) {
        let logo = |equipe: &str| if equipe == EQUIPE { ANCIEN } else { AUTRE };
        sqlx::query(
            "INSERT INTO competition_match_display_proj
               (pairing_id, season_id, round_id, round_name, round_position, round_day_type,
                home_team_id, home_team_name, home_roster_name, home_coach_name,
                home_logo_url, home_initials,
                away_team_id, away_team_name, away_roster_name, away_coach_name,
                away_logo_url, away_initials)
             VALUES ($1, 'saison', 'journee', 'J1', 1, 'regular',
                     $2, 'Dom', 'Roster', 'Coach', $3, 'DO',
                     $4, 'Ext', 'Roster', 'Coach', $5, 'EX')",
        )
        .bind(pairing_id)
        .bind(domicile)
        .bind(logo(domicile))
        .bind(exterieur)
        .bind(logo(exterieur))
        .execute(pool)
        .await
        .expect("match semé");
    }

    async fn logos(pool: &PgPool, pairing_id: &str) -> (Option<String>, Option<String>) {
        sqlx::query_as(
            "SELECT home_logo_url, away_logo_url FROM competition_match_display_proj
             WHERE pairing_id = $1",
        )
        .bind(pairing_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn evenement(logo_url: Option<&str>) -> TeamsAppEvent {
        TeamsAppEvent::LogoChanged {
            event_id: EventId::new(),
            team_id: TeamId::try_new(EQUIPE).unwrap(),
            space_id: SpaceId::new(),
            logo_url: logo_url.map(str::to_string),
        }
    }

    /// L'équipe est à domicile dans un match, à l'extérieur dans l'autre : son
    /// côté change dans les deux, celui de l'adversaire dans aucun.
    #[sqlx::test]
    async fn le_nouveau_logo_remplace_l_ancien_des_deux_cotes(pool: PgPool) {
        semer_match(&pool, "match-domicile", EQUIPE, ADVERSAIRE).await;
        semer_match(&pool, "match-exterieur", ADVERSAIRE, EQUIPE).await;

        mettre_a_jour_le_logo(evenement(Some(NOUVEAU)), &pool).await;

        assert_eq!(
            logos(&pool, "match-domicile").await,
            (Some(NOUVEAU.to_string()), Some(AUTRE.to_string()))
        );
        assert_eq!(
            logos(&pool, "match-exterieur").await,
            (Some(AUTRE.to_string()), Some(NOUVEAU.to_string()))
        );
    }

    /// Un logo retiré ne doit plus s'afficher : la projection repasse à NULL,
    /// comme `team_proj`.
    #[sqlx::test]
    async fn un_logo_retire_s_efface_de_la_projection(pool: PgPool) {
        semer_match(&pool, "match", EQUIPE, ADVERSAIRE).await;

        mettre_a_jour_le_logo(evenement(None), &pool).await;

        assert_eq!(logos(&pool, "match").await, (None, Some(AUTRE.to_string())));
    }

    /// Les matchs où l'équipe ne joue pas ne bougent pas.
    #[sqlx::test]
    async fn les_matchs_d_autres_equipes_ne_sont_pas_touches(pool: PgPool) {
        semer_match(&pool, "autre-match", ADVERSAIRE, ADVERSAIRE).await;

        mettre_a_jour_le_logo(evenement(Some(NOUVEAU)), &pool).await;

        assert_eq!(
            logos(&pool, "autre-match").await,
            (Some(AUTRE.to_string()), Some(AUTRE.to_string()))
        );
    }

    /// Le listener ne réagit qu'à son type d'événement : le filtre est sur
    /// `envelope.event_type`, avant toute désérialisation.
    #[sqlx::test]
    async fn un_autre_type_d_evenement_ne_declenche_rien(pool: PgPool) {
        semer_match(&pool, "match", EQUIPE, ADVERSAIRE).await;

        let bus = new_bus();
        init(&bus, pool.clone());
        let mut enveloppe = evenement(Some(NOUVEAU)).to_enveloppe();
        enveloppe.event_type = TeamsAppEvent::PLAYER_DISMISSED.to_string();
        let _ = bus.send(enveloppe); // arch:ok test — envoi direct pour éprouver le filtre
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }

        assert_eq!(
            logos(&pool, "match").await,
            (Some(ANCIEN.to_string()), Some(AUTRE.to_string()))
        );
    }
}
