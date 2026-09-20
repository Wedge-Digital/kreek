//! Les appariements de GBLR J11 après la réponse du commissaire (carte 558).
//!
//! # Ce qu'elle répare
//!
//! Sur la Journée 11 de GBLR Championnat, Ork'Lympic Rusé était apparié deux
//! fois — contre Lady's Ghosts et contre Les loups rouges — par deux saisies à
//! treize secondes d'intervalle, avant la garde de la carte 551. Les voix des
//! sables, seule équipe enrôlée sans match ce jour-là, était l'adversaire
//! manquant. Le commissaire a tranché : Les voix des sables reçoit Les loups
//! rouges.
//!
//! # Pourquoi recibler plutôt que supprimer et recréer
//!
//! L'appariement garde son identifiant et son brouillon de rapport. Le brouillon
//! suit par `SelectionUpdated`, l'événement que l'application écrit quand un
//! coach change la sélection : l'historique reste vrai, aucun événement n'est
//! réécrit.
//!
//! # Une migration à identifiants en dur
//!
//! C'est la première du registre à ne viser qu'une ligne connue. Elle ne fait
//! rien si l'état n'est plus celui attendu — l'appariement déjà reciblé, ou
//! Les voix des sables engagée entre-temps — et le dit en `warn`. Sur toute
//! autre base, elle ne trouve rien et passe.

use crate::common::initials::initials;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

pub struct GblrJ11;

pub const JOURNEE: &str = "01M1CJPKKF6ZSN3V4DFXM96J1A";
pub const APPARIEMENT: &str = "01M1CKS3FXS091ZXWNN94030MM";
pub const RAPPORT: &str = "01M1CKS3HFFGTNEFR87GQVBX3T";
pub const LOUPS_ROUGES: &str = "01M0YTDDGN0W83CPG7AH9XS0FY";
pub const ORK_LYMPIC: &str = "01M0YS7XST8HWPS8MN0GW2DDPQ";
pub const VOIX_DES_SABLES: &str = "01M0TJXC64AA71779Z57DG71CK";

#[async_trait]
impl DataMigration for GblrJ11 {
    fn nom(&self) -> &'static str {
        "558-gblr-j11"
    }

    async fn executer(
        &self,
        _state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        if !etat_attendu(tx).await? {
            tracing::warn!("GBLR J11 : l'appariement n'est plus dans l'état attendu, rien à faire");
            return Ok(0);
        }
        recibler_l_appariement(tx).await?;
        recibler_l_affichage(tx).await?;
        recibler_le_brouillon(tx).await?;
        tracing::info!(
            pairing_id = APPARIEMENT,
            "GBLR J11 : Les voix des sables reçoit Les loups rouges"
        );
        Ok(1)
    }
}

/// L'appariement porte encore Les loups rouges contre Ork'Lympic Rusé sur la
/// J11, Les voix des sables n'y joue pas, et le brouillon est encore `Draft`.
async fn etat_attendu(tx: &mut Transaction<'_, Postgres>) -> Result<bool, String> {
    let appariement: bool = sqlx::query_scalar(
        "SELECT exists(SELECT 1 FROM competition_match_day_pairings
                       WHERE id = $1 AND match_day_id = $2
                         AND home_team_id = $3 AND away_team_id = $4)",
    )
    .bind(APPARIEMENT)
    .bind(JOURNEE)
    .bind(LOUPS_ROUGES)
    .bind(ORK_LYMPIC)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture de l'appariement : {e}"))?;

    let voix_libre: bool = sqlx::query_scalar(
        "SELECT NOT exists(SELECT 1 FROM competition_match_day_pairings
                           WHERE match_day_id = $1
                             AND (home_team_id = $2 OR away_team_id = $2))",
    )
    .bind(JOURNEE)
    .bind(VOIX_DES_SABLES)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("recherche d'un engagement des Voix des sables : {e}"))?;

    let brouillon: bool = sqlx::query_scalar(
        "SELECT exists(SELECT 1 FROM match_report_proj
                       WHERE match_report_id = $1 AND pairing_id = $2 AND phase = 'Draft')",
    )
    .bind(RAPPORT)
    .bind(APPARIEMENT)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture du brouillon : {e}"))?;

    Ok(appariement && voix_libre && brouillon)
}

async fn recibler_l_appariement(tx: &mut Transaction<'_, Postgres>) -> Result<(), String> {
    sqlx::query(
        "UPDATE competition_match_day_pairings
         SET home_team_id = $2, away_team_id = $3
         WHERE id = $1",
    )
    .bind(APPARIEMENT)
    .bind(VOIX_DES_SABLES)
    .bind(LOUPS_ROUGES)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("reciblage de l'appariement : {e}"))?;
    Ok(())
}

/// Les deux camps sont recopiés depuis `team_proj`, initiales comprises —
/// calculées par la même fonction que l'application, et non approchées.
async fn recibler_l_affichage(tx: &mut Transaction<'_, Postgres>) -> Result<(), String> {
    let voix = equipe(tx, VOIX_DES_SABLES).await?;
    let loups = equipe(tx, LOUPS_ROUGES).await?;
    sqlx::query(
        "UPDATE competition_match_display_proj
         SET home_team_id = $2, home_team_name = $3, home_roster_name = $4,
             home_coach_name = $5, home_logo_url = $6, home_initials = $7,
             away_team_id = $8, away_team_name = $9, away_roster_name = $10,
             away_coach_name = $11, away_logo_url = $12, away_initials = $13
         WHERE pairing_id = $1",
    )
    .bind(APPARIEMENT)
    .bind(VOIX_DES_SABLES)
    .bind(&voix.team_name)
    .bind(&voix.roster_name)
    .bind(&voix.coach_name)
    .bind(&voix.logo_url)
    .bind(initials(&voix.team_name))
    .bind(LOUPS_ROUGES)
    .bind(&loups.team_name)
    .bind(&loups.roster_name)
    .bind(&loups.coach_name)
    .bind(&loups.logo_url)
    .bind(initials(&loups.team_name))
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("reciblage de la ligne d'affichage : {e}"))?;
    Ok(())
}

struct Equipe {
    team_name: String,
    roster_name: String,
    coach_name: String,
    logo_url: Option<String>,
}

async fn equipe(tx: &mut Transaction<'_, Postgres>, team_id: &str) -> Result<Equipe, String> {
    let row = sqlx::query(
        "SELECT team_name, roster_name, coach_name, logo_url FROM team_proj WHERE team_id = $1",
    )
    .bind(team_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture de l'équipe {team_id} : {e}"))?;
    Ok(Equipe {
        team_name: row.get("team_name"),
        roster_name: row.get("roster_name"),
        coach_name: row.get("coach_name"),
        logo_url: row.get("logo_url"),
    })
}

/// `SelectionUpdated`, tel que l'application l'écrit : l'événement d'abord, la
/// projection ensuite. L'auteur est celui de `MatchReportCreated` — la clé que
/// `competitions` pose sur un rapport né d'un appariement.
async fn recibler_le_brouillon(tx: &mut Transaction<'_, Postgres>) -> Result<(), String> {
    let (auteur, version): (String, i64) = sqlx::query_as(
        "SELECT (SELECT payload->>'created_by' FROM match_report_event_store
                 WHERE match_report_id = $1 AND event_type = 'MatchReportCreated'),
                coalesce(max(version), 0) + 1
         FROM match_report_event_store WHERE match_report_id = $1",
    )
    .bind(RAPPORT)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture du brouillon {RAPPORT} : {e}"))?;

    sqlx::query(
        "INSERT INTO match_report_event_store (match_report_id, event_type, payload, version)
         VALUES ($1, 'SelectionUpdated', $2, $3)",
    )
    .bind(RAPPORT)
    .bind(serde_json::json!({
        "type": "SelectionUpdated",
        "home_team_id": VOIX_DES_SABLES,
        "away_team_id": LOUPS_ROUGES,
        "updated_by": auteur,
    }))
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("append de SelectionUpdated : {e}"))?;

    sqlx::query(
        "UPDATE match_report_proj
         SET home_team_id = $2, away_team_id = $3, version = $4, updated_at = now()
         WHERE match_report_id = $1",
    )
    .bind(RAPPORT)
    .bind(VOIX_DES_SABLES)
    .bind(LOUPS_ROUGES)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("mise à jour de la projection du brouillon : {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::data_migrations::appliquer;

    async fn etat(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    /// La J11 telle que la production la portait : l'appariement fautif, sa
    /// ligne d'affichage, son brouillon, et les trois équipes.
    async fn semer(pool: &sqlx::PgPool) {
        let mut tx = pool.begin().await.unwrap();
        for (team_id, nom, roster, coach) in [
            (
                LOUPS_ROUGES,
                "Les loups rouges de Mideinheim",
                "Alliance du Vieux Monde",
                "frankygno",
            ),
            (ORK_LYMPIC, "Ork'Lympic Rusé", "Orques Noirs", "Ornox"),
            (
                VOIX_DES_SABLES,
                "Les voix des sables",
                "Rois des Tombes",
                "merlinc",
            ),
        ] {
            sqlx::query(
                "INSERT INTO team_proj (team_id, space_id, team_name, coach_name, roster_name, game_phase)
                 VALUES ($1, 'espace', $2, $4, $3, 'ReadyToPlay')",
            )
            .bind(team_id)
            .bind(nom)
            .bind(roster)
            .bind(coach)
            .execute(&mut *tx)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO competition_match_days (id, season_id, name, position)
             VALUES ($1, 'saison', 'Journée 11', 10)",
        )
        .bind(JOURNEE)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(APPARIEMENT)
        .bind(JOURNEE)
        .bind(LOUPS_ROUGES)
        .bind(ORK_LYMPIC)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO competition_match_display_proj (
                pairing_id, season_id, round_id, round_name, round_position, round_day_type,
                home_team_id, home_team_name, home_roster_name, home_coach_name, home_initials,
                away_team_id, away_team_name, away_roster_name, away_coach_name, away_initials,
                match_status)
             VALUES ($1, 'saison', $2, 'Journée 11', 10, 'time_frame',
                     $3, 'Les loups rouges de Mideinheim', 'Alliance du Vieux Monde', 'frankygno', 'LL',
                     $4, 'Ork''Lympic Rusé', 'Orques Noirs', 'Ornox', 'OR',
                     'upcoming')",
        )
        .bind(APPARIEMENT)
        .bind(JOURNEE)
        .bind(LOUPS_ROUGES)
        .bind(ORK_LYMPIC)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO match_report_event_store (match_report_id, event_type, payload, version)
             VALUES ($1, 'MatchReportCreated', $2, 1)",
        )
        .bind(RAPPORT)
        .bind(serde_json::json!({
            "type": "MatchReportCreated",
            "created_by": "competition",
            "pairing_id": APPARIEMENT,
            "round_id": JOURNEE,
        }))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO match_report_proj (match_report_id, space_id, competition_id, season_id,
                round_id, home_team_id, away_team_id, created_by, origin, phase, version, pairing_id)
             VALUES ($1, 'espace', 'competition', 'saison', $2, $3, $4, 'competition', 'Pairing',
                'Draft', 1, $5)",
        )
        .bind(RAPPORT)
        .bind(JOURNEE)
        .bind(LOUPS_ROUGES)
        .bind(ORK_LYMPIC)
        .bind(APPARIEMENT)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn camps(pool: &sqlx::PgPool) -> (String, String) {
        sqlx::query_as(
            "SELECT home_team_id, away_team_id FROM competition_match_day_pairings WHERE id = $1",
        )
        .bind(APPARIEMENT)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn le_second_adversaire_d_ornox_devient_les_voix_des_sables(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        semer(&pool).await;

        appliquer(&state, &pool, vec![Box::new(GblrJ11)])
            .await
            .unwrap();

        assert_eq!(
            camps(&pool).await,
            (VOIX_DES_SABLES.to_string(), LOUPS_ROUGES.to_string())
        );
        let affichage: (String, String, String, String) = sqlx::query_as(
            "SELECT home_team_name, home_initials, away_coach_name, away_initials
             FROM competition_match_display_proj WHERE pairing_id = $1",
        )
        .bind(APPARIEMENT)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            affichage,
            (
                "Les voix des sables".into(),
                "LV".into(),
                "frankygno".into(),
                "LL".into()
            )
        );
        let brouillon: (String, String, i64) = sqlx::query_as(
            "SELECT home_team_id, away_team_id, version FROM match_report_proj WHERE match_report_id = $1",
        )
        .bind(RAPPORT)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(brouillon, (VOIX_DES_SABLES.into(), LOUPS_ROUGES.into(), 2));
        let evenement: (String, serde_json::Value) = sqlx::query_as(
            "SELECT event_type, payload FROM match_report_event_store
             WHERE match_report_id = $1 AND version = 2",
        )
        .bind(RAPPORT)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(evenement.0, "SelectionUpdated");
        assert_eq!(evenement.1["updated_by"], "competition");
    }

    /// Rejouée après coup — ou sur une base où l'appariement n'a jamais eu
    /// cette forme — elle n'écrit rien.
    #[sqlx::test]
    async fn un_appariement_deja_recible_n_est_pas_touche(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        semer(&pool).await;
        sqlx::query("UPDATE competition_match_day_pairings SET away_team_id = $2 WHERE id = $1")
            .bind(APPARIEMENT)
            .bind(VOIX_DES_SABLES)
            .execute(&pool)
            .await
            .unwrap();

        appliquer(&state, &pool, vec![Box::new(GblrJ11)])
            .await
            .unwrap();

        assert_eq!(
            camps(&pool).await,
            (LOUPS_ROUGES.to_string(), VOIX_DES_SABLES.to_string())
        );
        let evenements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM match_report_event_store WHERE match_report_id = $1",
        )
        .bind(RAPPORT)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(evenements, 1, "aucun SelectionUpdated ajouté");
    }
}
