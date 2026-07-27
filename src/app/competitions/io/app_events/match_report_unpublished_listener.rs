use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, MatchReportUnpublishedPayload,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;

/// Remet la ligne de résultats/calendrier dans l'état où
/// `match_report_confirmed_listener` l'avait laissée : match en cours, sans
/// score, avec un lien vers la saisie.
pub fn init(app_event_bus: &EventBus, pool: PgPool) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportUnpublished(payload)) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    handle_unpublished(&payload, &pool).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_unpublished_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_unpublished(payload: &MatchReportUnpublishedPayload, pool: &PgPool) {
    // Sans pairing, il n'y a pas de ligne de projection à compenser. Le cas ne
    // devrait pas survenir — la publication en crée un pour les rapports
    // manuels — mais il n'appelle aucune alerte.
    let Some(pairing_id) = payload.pairing_id.as_deref() else {
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .edit_match_report(&payload.space_id, &payload.match_report_id);

    if let Err(e) = reset_projection(pool, pairing_id, &report_url).await {
        tracing::error!(
            "competitions::match_report_unpublished_listener: update {pairing_id}: {e}"
        );
    }
}

/// **Ne recrée aucun pairing**, contrairement au listener de publication qui en
/// crée un pour les rapports manuels : ici il existe déjà, et la re-publication
/// le retrouvera par son identifiant.
///
/// `UPDATE` à valeurs absolues sur une clé stable, donc naturellement
/// idempotent — c'est ce qui rend acceptable un rejeu de la compensation.
async fn reset_projection(
    pool:       &PgPool,
    pairing_id: &str,
    report_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status     = 'in_progress',
             home_score       = NULL,
             away_score       = NULL,
             home_casualties  = NULL,
             away_casualties  = NULL,
             match_report_url = $2
         WHERE pairing_id = $1",
        pairing_id,
        report_url,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    const PAIRING: &str = "pairing-1";

    fn payload(pairing_id: Option<&str>) -> MatchReportUnpublishedPayload {
        MatchReportUnpublishedPayload {
            match_report_id: "mr1".into(),
            space_id:        "sp1".into(),
            competition_id:  "c1".into(),
            season_id:       "s1".into(),
            round_id:        "r1".into(),
            pairing_id:      pairing_id.map(str::to_string),
            home_team_id:    "home".into(),
            away_team_id:    "away".into(),
            unpublished_at:  chrono::Utc::now(),
        }
    }

    /// Une ligne telle que la publication l'aurait laissée : match terminé,
    /// scores et sorties renseignés.
    async fn seed_completed_row(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO competition_match_display_proj (
                 pairing_id, season_id, round_id, round_name, round_position,
                 round_day_type, home_team_id, home_team_name, home_roster_name,
                 home_coach_name, home_initials, away_team_id, away_team_name,
                 away_roster_name, away_coach_name, away_initials,
                 match_status, home_score, away_score, home_casualties,
                 away_casualties, match_report_url
             ) VALUES ($1, 's1', 'r1', 'Journée 1', 0, 'FixedDate',
                       'home', 'Home', 'Orcs', 'C1', 'HOM',
                       'away', 'Away', 'Elfes', 'C2', 'AWA',
                       'completed', 2, 1, 3, 0, '/recap')",
        )
        .bind(PAIRING)
        .execute(pool)
        .await
        .expect("insertion de la ligne de test");
    }

    async fn row_status(pool: &PgPool) -> (String, Option<i32>, Option<i32>, Option<String>) {
        let row = sqlx::query(
            "SELECT match_status, home_score, home_casualties, match_report_url
             FROM competition_match_display_proj WHERE pairing_id = $1",
        )
        .bind(PAIRING)
        .fetch_one(pool)
        .await
        .unwrap();
        (
            row.get("match_status"),
            row.get("home_score"),
            row.get("home_casualties"),
            row.get("match_report_url"),
        )
    }

    #[sqlx::test]
    async fn la_compensation_remet_le_match_en_cours_et_efface_les_scores(pool: PgPool) {
        seed_completed_row(&pool).await;

        handle_unpublished(&payload(Some(PAIRING)), &pool).await;

        let (status, score, casualties, url) = row_status(&pool).await;
        assert_eq!(status, "in_progress");
        assert_eq!(score, None);
        assert_eq!(casualties, None);
        assert!(
            url.unwrap().ends_with("/match-report/mr1"),
            "l'URL doit pointer vers la saisie, pas le recap"
        );
    }

    /// Règle 11 : la compensation doit pouvoir être rejouée sans effet
    /// supplémentaire — c'est ce qui rend acceptable un échec partiel.
    #[sqlx::test]
    async fn deux_compensations_successives_donnent_le_meme_resultat(pool: PgPool) {
        seed_completed_row(&pool).await;

        handle_unpublished(&payload(Some(PAIRING)), &pool).await;
        let first = row_status(&pool).await;
        handle_unpublished(&payload(Some(PAIRING)), &pool).await;
        let second = row_status(&pool).await;

        assert_eq!(first, second);
    }

    /// Contrairement au listener de publication, la compensation ne crée jamais
    /// de pairing : il existe déjà, et la re-publication le retrouvera.
    #[sqlx::test]
    async fn aucun_pairing_n_est_cree(pool: PgPool) {
        seed_completed_row(&pool).await;
        let before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM competition_match_day_pairings")
                .fetch_one(&pool)
                .await
                .unwrap();

        handle_unpublished(&payload(Some(PAIRING)), &pool).await;

        let after: i64 =
            sqlx::query_scalar("SELECT count(*) FROM competition_match_day_pairings")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(before, after);
    }

    #[sqlx::test]
    async fn sans_pairing_rien_n_est_touche(pool: PgPool) {
        seed_completed_row(&pool).await;

        handle_unpublished(&payload(None), &pool).await;

        let (status, score, _, _) = row_status(&pool).await;
        assert_eq!(status, "completed");
        assert_eq!(score, Some(2));
    }
}
