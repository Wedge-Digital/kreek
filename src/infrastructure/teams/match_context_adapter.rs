//! Le contexte d'un match, lu chez `competitions` (carte 435).
//!
//! **Le seul fichier de `teams` qui importe `competitions`** — et il ne
//! l'importe pas non plus : il lit sa projection d'affichage directement, comme
//! les autres adapters de ce dossier.
//!
//! `competition_match_display_proj` porte le nom de journée, les deux équipes et
//! le score **sur une seule ligne**, indexée par `match_report_id`. C'est ce qui
//! permet une requête par match plutôt qu'une jointure de trois tables.

use crate::app::teams::ports::{IMatchContextPort, MatchContextDto};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct MatchContextAdapter {
    pool: PgPool,
}

impl MatchContextAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IMatchContextPort for MatchContextAdapter {
    /// `None` quand le match n'a **aucune ligne d'affichage**, pas quand il est
    /// en cours — un match en cours a sa ligne, avec des scores à `NULL`.
    ///
    /// Une erreur de lecture rend `None` elle aussi : le relevé perd le détail
    /// d'une ligne, il ne s'arrête pas. Un mouvement sans son contexte reste un
    /// mouvement juste ; un relevé absent ne l'est pas.
    async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto> {
        #[derive(sqlx::FromRow)]
        struct Row {
            round_name: String,
            home_team_id: String,
            home_team_name: String,
            away_team_id: String,
            away_team_name: String,
            home_score: Option<i32>,
            away_score: Option<i32>,
        }

        let row = sqlx::query_as::<_, Row>(
            "SELECT round_name, home_team_id, home_team_name,
                    away_team_id, away_team_name, home_score, away_score
             FROM   competition_match_display_proj
             WHERE  match_report_id = $1",
        )
        .bind(match_report_id)
        .fetch_optional(&self.pool)
        .await
        .inspect_err(|e| tracing::warn!(%match_report_id, "contexte de match illisible : {e}"))
        .ok()
        .flatten()?;

        Some(MatchContextDto {
            round_name: row.round_name,
            home_team_id: row.home_team_id,
            home_team_name: row.home_team_name,
            away_team_id: row.away_team_id,
            away_team_name: row.away_team_name,
            home_score: row.home_score.map(|s| s as u8),
            away_score: row.away_score.map(|s| s as u8),
        })
    }
}
