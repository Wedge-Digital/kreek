//! Les rencontres qui portent deux rapports vivants (carte 555).
//!
//! # Ce qu'elle répare
//!
//! Le chemin hors calendrier créait un appariement, dont l'événement faisait
//! créer un rapport par `pairing_created_listener` — pendant que le contrôleur
//! en créait un second. Les deux se dédupliquaient en relisant la base, et
//! chacun lisait avant que l'autre n'écrive.
//!
//! Relevé après une seule exécution de la suite e2e : **38 appariements à deux
//! rapports vivants**. La carte 555 supprime la cause ; celle-ci nettoie ce
//! qu'elle a laissé.
//!
//! # Lequel garder
//!
//! **Le plus avancé.** Un rapport publié porte un résultat au classement, un
//! rapport prêt à publier porte une saisie entière : les perdre pour garder un
//! brouillon vide serait détruire du travail de coach.
//!
//! À phase égale, le **plus ancien** — c'est celui que les écrans ont montré,
//! donc celui que les gens croient avoir rempli.
//!
//! # Ce qu'elle ne fait pas
//!
//! Elle ne libère pas les équipes : le rapport conservé les occupe
//! légitimement, et les rendre disponibles les décrocherait d'une saisie en
//! cours. C'est la différence avec `m003`, qui annulait un rapport sans
//! remplaçant.

use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

pub struct RapportsEnDouble;

/// Un rapport à annuler, et la rencontre qu'il doublonne.
struct EnTrop {
    match_report_id: String,
    home_team_id: String,
    away_team_id: String,
}

#[async_trait]
impl DataMigration for RapportsEnDouble {
    fn nom(&self) -> &'static str {
        "555-rapports-en-double"
    }

    async fn executer(
        &self,
        _state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        let mut touches = 0;
        for d in doublons(tx).await? {
            annuler(tx, &d).await?;
            touches += 1;
        }
        Ok(touches)
    }
}

/// Les rapports en trop : tous sauf le survivant de chaque rencontre.
///
/// Le classement se fait en SQL plutôt qu'en Rust parce que le critère est un
/// ordre — le plus avancé, puis le plus ancien — et qu'une fenêtre l'exprime
/// sans charger les agrégats.
async fn doublons(tx: &mut Transaction<'_, Postgres>) -> Result<Vec<EnTrop>, String> {
    let rows = sqlx::query(
        "WITH classes AS (
             SELECT match_report_id, home_team_id, away_team_id,
                    row_number() OVER (
                        PARTITION BY pairing_id
                        ORDER BY CASE phase
                                   WHEN 'Published'      THEN 0
                                   WHEN 'ReadyToPublish' THEN 1
                                   WHEN 'PreMatch'       THEN 2
                                   ELSE 3
                                 END,
                                 created_at
                    ) AS rang
             FROM match_report_proj
             WHERE phase <> 'Cancelled' AND pairing_id IS NOT NULL
         )
         SELECT match_report_id, home_team_id, away_team_id FROM classes WHERE rang > 1",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| format!("lecture des rapports en double : {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| EnTrop {
            match_report_id: r.get("match_report_id"),
            home_team_id: r.get("home_team_id"),
            away_team_id: r.get("away_team_id"),
        })
        .collect())
}

/// Annule le rapport en trop : l'événement d'abord, la projection ensuite.
async fn annuler(tx: &mut Transaction<'_, Postgres>, d: &EnTrop) -> Result<(), String> {
    let version: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(version), 0) + 1 FROM match_report_event_store
         WHERE match_report_id = $1",
    )
    .bind(&d.match_report_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("lecture de la version : {e}"))?;

    sqlx::query(
        "INSERT INTO match_report_event_store (match_report_id, event_type, payload, version)
         VALUES ($1, 'MatchReportCancelled', $2, $3)",
    )
    .bind(&d.match_report_id)
    .bind(serde_json::json!({
        "type": "MatchReportCancelled",
        "reason": "Rapport en double sur la même rencontre (carte 555)",
        "journeymen": [],
        "pairing_id": null,
        "home_team_id": d.home_team_id,
        "away_team_id": d.away_team_id,
    }))
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("append de l'annulation : {e}"))?;

    sqlx::query(
        "UPDATE match_report_proj SET phase = 'Cancelled', version = $2
         WHERE match_report_id = $1",
    )
    .bind(&d.match_report_id)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("annulation dans la projection : {e}"))?;

    tracing::info!(
        match_report_id = %d.match_report_id,
        "rapport en double annulé : la rencontre en gardait un plus avancé"
    );
    Ok(())
}
