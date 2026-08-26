//! Défait, côté calendrier, ce que la confirmation d'un rapport avait posé.
//!
//! Deux cas, et c'est `pairing_id` qui les sépare — l'appariement que le rapport
//! portait **à sa création** :
//!
//! - **`Some`, rencontre programmée** : l'appariement appartient au calendrier
//!   et y reste. Seule sa ligne d'affichage repasse en « à venir », débarrassée
//!   du lien vers un rapport qui n'existe plus.
//! - **`None`, rapport manuel** : l'appariement n'a été fabriqué que pour lui
//!   (carte 427). Il s'en va avec lui, sa ligne comprise.
//!
//! Sans ce listener, une annulation laissait la ligne figée sur « en cours »,
//! pointant un rapport annulé — pour toujours.

use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, pool: PgPool, match_day_repo: Arc<dyn IMatchDayRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_event(event, &pool, match_day_repo.as_ref())
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_cancelled_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(
    event: MatchReportAppEvent,
    pool: &PgPool,
    match_day_repo: &dyn IMatchDayRepository,
) {
    // Sortie silencieuse : le bus porte tous les app events de l'application.
    // Celles qui suivent laissent une ligne.
    let MatchReportAppEvent::MatchReportCancelled {
        match_report_id,
        pairing_id,
        ..
    } = event
    else {
        return;
    };

    match pairing_id {
        Some(pairing_id) => remettre_a_venir(pool, &pairing_id, &match_report_id).await,
        None => supprimer_appariement_manuel(pool, match_day_repo, &match_report_id).await,
    }
}

/// Rencontre programmée : l'appariement reste, sa ligne redevient « à venir ».
async fn remettre_a_venir(pool: &PgPool, pairing_id: &str, match_report_id: &str) {
    let r = sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status     = 'upcoming',
             match_report_id  = NULL,
             match_report_url = NULL
         WHERE pairing_id = $1",
        pairing_id,
    )
    .execute(pool)
    .await;

    match r {
        Err(e) => tracing::error!(
            "competitions::match_report_cancelled_listener: remise à venir {pairing_id}: {e}"
        ),
        Ok(r) if r.rows_affected() == 0 => tracing::warn!(
            pairing_id = %pairing_id,
            match_report_id = %match_report_id,
            "annulation sans effet : aucune ligne d'affichage pour cet appariement"
        ),
        Ok(_) => {}
    }
}

/// Rapport manuel : l'appariement n'existait que pour lui.
///
/// Son identifiant n'est pas dans l'événement — le rapport n'en portait aucun —
/// mais la ligne d'affichage le connaît, la confirmation l'y ayant inscrit.
async fn supprimer_appariement_manuel(
    pool: &PgPool,
    match_day_repo: &dyn IMatchDayRepository,
    match_report_id: &str,
) {
    let trouve = sqlx::query_scalar!(
        "SELECT pairing_id FROM competition_match_display_proj WHERE match_report_id = $1",
        match_report_id,
    )
    .fetch_optional(pool)
    .await;

    let pairing_id = match trouve {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(
                match_report_id = %match_report_id,
                "annulation d'un rapport manuel sans ligne d'affichage : rien à défaire"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                "competitions::match_report_cancelled_listener: recherche de l'appariement de {match_report_id}: {e}"
            );
            return;
        }
    };

    // La ligne d'abord : elle référence l'appariement, et la supprimer ensuite
    // laisserait une fenêtre où elle pointerait dans le vide.
    if let Err(e) = sqlx::query!(
        "DELETE FROM competition_match_display_proj WHERE pairing_id = $1",
        pairing_id,
    )
    .execute(pool)
    .await
    {
        tracing::error!(
            "competitions::match_report_cancelled_listener: suppression de la ligne {pairing_id}: {e}"
        );
        return;
    }

    if let Err(e) = match_day_repo.delete_pairing(&pairing_id).await {
        tracing::error!(
            "competitions::match_report_cancelled_listener: suppression de l'appariement {pairing_id}: {e:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
    };
    use crate::app::competitions::domain::match_day_repository_port::NewPairingProjection;
    use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::EventId;

    const MR: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";

    /// Une rencontre au calendrier, sa ligne d'affichage, et un rapport en cours
    /// dessus — l'état que la confirmation laisse. Retourne l'appariement.
    async fn rencontre_en_cours(pool: &PgPool) -> String {
        let jour = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 7".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(3).unwrap(),
            pairings: vec![],
        };
        let repo = MatchDayRepository::new(pool.clone());
        repo.save_match_day(&jour).await.unwrap();

        let pairing = Pairing {
            id: PairingId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
        };
        let projection = NewPairingProjection {
            season_id: jour.season_id.to_string(),
            round_name: jour.name.to_string(),
            round_position: jour.position.into_inner(),
            round_date_start: None,
            round_date_end: None,
            round_day_type: jour.day_type.as_str().to_string(),
            home_team_name: "Home".into(),
            home_roster_name: "R".into(),
            home_coach_name: "C1".into(),
            home_logo_url: None,
            away_team_name: "Away".into(),
            away_roster_name: "R".into(),
            away_coach_name: "C2".into(),
            away_logo_url: None,
        };
        repo.save_pairing(&jour.id.to_string(), &pairing, &projection)
            .await
            .unwrap();

        let pairing_id = pairing.id.to_string();
        sqlx::query!(
            "UPDATE competition_match_display_proj
             SET match_status = 'in_progress', match_report_id = $2
             WHERE pairing_id = $1",
            pairing_id,
            MR,
        )
        .execute(pool)
        .await
        .unwrap();
        pairing_id
    }

    fn annulation(pairing_id: Option<String>) -> MatchReportAppEvent {
        MatchReportAppEvent::MatchReportCancelled {
            event_id: EventId::new(),
            match_report_id: MR.to_string(),
            home_team_id: TeamId::new().to_string(),
            away_team_id: TeamId::new().to_string(),
            pairing_id,
        }
    }

    async fn statut(pool: &PgPool, pairing_id: &str) -> Option<String> {
        sqlx::query_scalar!(
            "SELECT match_status FROM competition_match_display_proj WHERE pairing_id = $1",
            pairing_id,
        )
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    async fn appariement_existe(pool: &PgPool, pairing_id: &str) -> bool {
        sqlx::query_scalar!(
            "SELECT count(*) FROM competition_match_day_pairings WHERE id = $1",
            pairing_id,
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap_or(0)
            > 0
    }

    /// L'appariement d'un rapport manuel n'existait que pour lui : il s'en va
    /// avec lui, sa ligne comprise.
    #[sqlx::test]
    async fn l_annulation_d_un_rapport_manuel_supprime_son_appariement(pool: PgPool) {
        let pairing_id = rencontre_en_cours(&pool).await;
        let repo = MatchDayRepository::new(pool.clone());

        handle_event(annulation(None), &pool, &repo).await;

        assert!(
            statut(&pool, &pairing_id).await.is_none(),
            "la ligne doit partir"
        );
        assert!(
            !appariement_existe(&pool, &pairing_id).await,
            "l'appariement fabriqué pour ce rapport doit partir aussi"
        );
    }

    /// Une rencontre programmée appartient au calendrier : elle y reste, et
    /// redevient simplement « à venir ». La supprimer effacerait un match que
    /// l'administrateur avait programmé.
    #[sqlx::test]
    async fn l_annulation_d_un_rapport_programme_le_remet_en_upcoming(pool: PgPool) {
        let pairing_id = rencontre_en_cours(&pool).await;
        let repo = MatchDayRepository::new(pool.clone());

        handle_event(annulation(Some(pairing_id.clone())), &pool, &repo).await;

        assert_eq!(
            statut(&pool, &pairing_id).await.as_deref(),
            Some("upcoming")
        );
        assert!(
            appariement_existe(&pool, &pairing_id).await,
            "la rencontre programmée doit rester au calendrier"
        );
    }
}
