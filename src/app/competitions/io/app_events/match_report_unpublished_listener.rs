use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, MatchReportUnpublishedPayload,
};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use tracing::Instrument;

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
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_unpublished(&payload, &pool).instrument(span).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        "competitions::match_report_unpublished_listener: lagged by {n}"
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_unpublished(payload: &MatchReportUnpublishedPayload, pool: &PgPool) {
    let Some(pairing_id) = resolve_pairing_id(payload, pool).await else {
        tracing::warn!(
            "competitions::match_report_unpublished_listener: aucun pairing pour {} — \
             ligne de calendrier non compensée",
            payload.match_report_id
        );
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .edit_match_report(&payload.space_id, &payload.match_report_id);

    if let Err(e) = reset_projection(pool, &pairing_id, &report_url).await {
        tracing::error!(
            "competitions::match_report_unpublished_listener: update {pairing_id}: {e}"
        );
    }
}

/// Retrouve le pairing dont la ligne de calendrier doit être compensée.
///
/// Un rapport **manuel** n'en porte pas : c'est la publication qui en a créé un
/// (cf. `resolve_pairing_id` du listener de publication), sans que l'agrégat du
/// rapport en soit informé. Son identifiant n'est donc pas dans le payload, et
/// s'arrêter là laisserait le match affiché « terminé » avec un lien vers le
/// récapitulatif d'un rapport qui n'est plus publié.
///
/// On le retrouve par la même clé que celle qui a servi à le créer : journée et
/// équipes.
async fn resolve_pairing_id(
    payload: &MatchReportUnpublishedPayload,
    pool: &PgPool,
) -> Option<String> {
    if let Some(pairing_id) = payload.pairing_id.clone() {
        return Some(pairing_id);
    }

    sqlx::query_scalar!(
        "SELECT id FROM competition_match_day_pairings
         WHERE match_day_id = $1 AND home_team_id = $2 AND away_team_id = $3
         LIMIT 1",
        payload.round_id,
        payload.home_team_id,
        payload.away_team_id,
    )
    .fetch_optional(pool)
    .await
    .unwrap_or_else(|e| {
        tracing::error!(
            "competitions::match_report_unpublished_listener: résolution du pairing : {e}"
        );
        None
    })
}

/// **Ne recrée aucun pairing**, contrairement au listener de publication qui en
/// crée un pour les rapports manuels : ici il existe déjà, et la re-publication
/// le retrouvera par son identifiant.
///
/// `UPDATE` à valeurs absolues sur une clé stable, donc naturellement
/// idempotent — c'est ce qui rend acceptable un rejeu de la compensation.
async fn reset_projection(
    pool: &PgPool,
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
            space_id: "sp1".into(),
            competition_id: "c1".into(),
            season_id: "s1".into(),
            round_id: "r1".into(),
            pairing_id: pairing_id.map(str::to_string),
            home_team_id: "home".into(),
            away_team_id: "away".into(),
            unpublished_at: chrono::Utc::now(),
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

    /// Le pairing tel que la publication d'un rapport manuel l'aurait créé —
    /// mêmes journée et équipes que le payload.
    async fn seed_pairing(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO competition_match_days (id, season_id, name, day_type, position)
             VALUES ('r1', 's1', 'Journée 1', 'FixedDate', 0)
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(pool)
        .await
        .expect("insertion de la journée de test");

        sqlx::query(
            "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
             VALUES ($1, 'r1', 'home', 'away')",
        )
        .bind(PAIRING)
        .execute(pool)
        .await
        .expect("insertion du pairing de test");
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
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM competition_match_day_pairings")
            .fetch_one(&pool)
            .await
            .unwrap();

        handle_unpublished(&payload(Some(PAIRING)), &pool).await;

        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM competition_match_day_pairings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before, after);
    }

    /// Régression trouvée en exerçant la feature pour de vrai : un rapport
    /// **manuel** ne porte pas de `pairing_id` — c'est la publication qui a créé
    /// le pairing, sans en informer l'agrégat du rapport. S'arrêter au payload
    /// laissait le match affiché « terminé » avec un lien vers le récapitulatif
    /// d'un rapport qui n'était plus publié.
    #[sqlx::test]
    async fn un_rapport_manuel_est_compense_via_le_pairing_retrouve(pool: PgPool) {
        seed_completed_row(&pool).await;
        seed_pairing(&pool).await;

        // payload sans pairing_id, comme pour un rapport créé hors calendrier
        handle_unpublished(&payload(None), &pool).await;

        let (status, score, _, url) = row_status(&pool).await;
        assert_eq!(status, "in_progress");
        assert_eq!(score, None);
        assert!(url.unwrap().ends_with("/match-report/mr1"));
    }

    #[sqlx::test]
    async fn sans_pairing_retrouvable_rien_n_est_touche(pool: PgPool) {
        seed_completed_row(&pool).await;

        handle_unpublished(&payload(None), &pool).await;

        let (status, score, _, _) = row_status(&pool).await;
        assert_eq!(status, "completed");
        assert_eq!(score, Some(2));
    }
}
