use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, MatchActionRow, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::{rehydrate, MatchReportState};
use crate::app::match_report::domain::value_objects::TeamSide;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct MatchReportRepository {
    pool: PgPool,
}

impl MatchReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn update_projection_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        match_report_id: &str,
        event: &MatchReportDomainEvent,
        version: u64,
    ) -> Result<(), RepositoryError> {
        match event {
            MatchReportDomainEvent::MatchReportCreated {
                space_id,
                competition_id,
                season_id,
                round_id,
                home_team_id,
                away_team_id,
                created_by,
                origin,
                pairing_id,
                ..
            } => {
                let origin_str = match origin {
                    crate::app::match_report::domain::value_objects::MatchReportOrigin::Manual => {
                        "Manual"
                    }
                    crate::app::match_report::domain::value_objects::MatchReportOrigin::Pairing => {
                        "Pairing"
                    }
                };
                sqlx::query(
                    "INSERT INTO match_report_projection
                        (match_report_id, space_id, competition_id, season_id, round_id,
                         home_team_id, away_team_id, created_by, origin, phase, version, pairing_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Draft', $10, $11)
                     ON CONFLICT (match_report_id) DO NOTHING",
                )
                .bind(match_report_id)
                .bind(space_id.to_string())
                .bind(competition_id.to_string())
                .bind(season_id.to_string())
                .bind(round_id.to_string())
                .bind(home_team_id.to_string())
                .bind(away_team_id.to_string())
                .bind(created_by.to_string())
                .bind(origin_str)
                .bind(version as i64)
                .bind(pairing_id.as_deref())
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::SelectionUpdated {
                home_team_id,
                away_team_id,
                ..
            } => {
                sqlx::query(
                    "UPDATE match_report_projection
                     SET home_team_id = $2, away_team_id = $3, version = $4, updated_at = now()
                     WHERE match_report_id = $1",
                )
                .bind(match_report_id)
                .bind(home_team_id.to_string())
                .bind(away_team_id.to_string())
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::SelectionConfirmed { .. } => {
                sqlx::query(
                    "UPDATE match_report_projection
                     SET phase = 'PreMatch', version = $2, updated_at = now()
                     WHERE match_report_id = $1",
                )
                .bind(match_report_id)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::FanFactorRecorded { .. } => {}
            MatchReportDomainEvent::TeamValuesRecorded { home_team_value, away_team_value, .. } => {
                sqlx::query(
                    "UPDATE match_report_projection
                     SET home_team_value = $2, away_team_value = $3, version = $4, updated_at = now()
                     WHERE match_report_id = $1",
                )
                .bind(match_report_id)
                .bind(home_team_value.into_inner() as i32)
                .bind(away_team_value.into_inner() as i32)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::InducementsRecorded { team_id, purchases, .. } => {
                let json = serde_json::to_value(purchases).map_err(RepositoryError::Serialization)?;
                let is_home = sqlx::query(
                    "SELECT home_team_id FROM match_report_projection WHERE match_report_id = $1",
                )
                .bind(match_report_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(RepositoryError::Database)
                .map(|r| r.get::<String, _>("home_team_id") == team_id.to_string())?;
                let col = if is_home { "home_inducements" } else { "away_inducements" };
                sqlx::query(&format!(
                    "UPDATE match_report_projection SET {col} = $2, version = $3, updated_at = now() WHERE match_report_id = $1"
                ))
                .bind(match_report_id)
                .bind(&json)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::StarPlayerEngaged { .. } => {}
            MatchReportDomainEvent::TempPlayersInitialized { team_id, players } => {
                let json = serde_json::to_value(players).map_err(RepositoryError::Serialization)?;
                let is_home = is_home_team(tx, match_report_id, &team_id.to_string()).await?;
                let col = if is_home { "home_temp_players" } else { "away_temp_players" };
                sqlx::query(&format!(
                    "UPDATE match_report_projection SET {col} = $2, version = $3, updated_at = now() WHERE match_report_id = $1"
                ))
                .bind(match_report_id)
                .bind(&json)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::TempPlayersReset { team_id } => {
                let is_home = is_home_team(tx, match_report_id, &team_id.to_string()).await?;
                let col = if is_home { "home_temp_players" } else { "away_temp_players" };
                sqlx::query(&format!(
                    "UPDATE match_report_projection SET {col} = '[]'::jsonb, version = $2, updated_at = now() WHERE match_report_id = $1"
                ))
                .bind(match_report_id)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::ActionRecorded {
                action_id, team_side, turn, player, action, player_display_name, player_position, ..
            } => {
                use crate::app::match_report::domain::value_objects::TeamSide;
                let (player_id, player_type) = match player {
                    crate::app::match_report::domain::value_objects::ActionPlayer::Regular(id) => (id.to_string(), "regular"),
                    crate::app::match_report::domain::value_objects::ActionPlayer::Temp(id) => (id.0.clone(), "temp"),
                };
                let action_json = serde_json::to_value(action).map_err(RepositoryError::Serialization)?;
                let side_str = match team_side { TeamSide::Home => "home", TeamSide::Away => "away" };
                sqlx::query(
                    "INSERT INTO match_report_actions
                        (action_id, match_report_id, team_side, turn_number, player_id,
                         player_type, action_json, player_display_name, player_position)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                )
                .bind(&action_id.0)
                .bind(match_report_id)
                .bind(side_str)
                .bind(turn.value() as i16)
                .bind(&player_id)
                .bind(player_type)
                .bind(&action_json)
                .bind(player_display_name)
                .bind(player_position)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::ActionDeleted { action_id, .. } => {
                sqlx::query(
                    "UPDATE match_report_actions SET is_deleted = TRUE WHERE action_id = $1",
                )
                .bind(&action_id.0)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            MatchReportDomainEvent::MatchReportCancelled { .. } => {
                sqlx::query(
                    "UPDATE match_report_projection
                     SET phase = 'Cancelled', version = $2, updated_at = now()
                     WHERE match_report_id = $1",
                )
                .bind(match_report_id)
                .bind(version as i64)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
        }
        Ok(())
    }
}

async fn is_home_team(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    match_report_id: &str,
    team_id: &str,
) -> Result<bool, RepositoryError> {
    let row = sqlx::query(
        "SELECT home_team_id FROM match_report_projection WHERE match_report_id = $1",
    )
    .bind(match_report_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(RepositoryError::Database)?;
    Ok(row.get::<String, _>("home_team_id") == team_id)
}

#[async_trait]
impl IMatchReportRepository for MatchReportRepository {
    async fn append(
        &self,
        match_report_id: &str,
        event: &MatchReportDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let new_version = expected_version + 1;
        let payload = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;
        let event_type = event.type_name();
        let event_version = event.schema_version();

        let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;

        sqlx::query(
            "INSERT INTO match_report_event_store
                (match_report_id, event_type, event_version, payload, version)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(match_report_id)
        .bind(event_type)
        .bind(event_version)
        .bind(&payload)
        .bind(new_version as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db) = e {
                if db.constraint() == Some("match_report_es_version") {
                    return RepositoryError::ConcurrentWrite;
                }
            }
            RepositoryError::Database(e)
        })?;

        self.update_projection_in_tx(&mut tx, match_report_id, event, new_version)
            .await?;

        tx.commit().await.map_err(RepositoryError::Database)?;
        Ok(new_version)
    }

    async fn find_by_id(
        &self,
        match_report_id: &str,
    ) -> Result<Option<MatchReportState>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT payload FROM match_report_event_store
             WHERE match_report_id = $1 ORDER BY version ASC",
        )
        .bind(match_report_id)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let events: Vec<MatchReportDomainEvent> = rows
            .iter()
            .map(|r| {
                let payload: serde_json::Value = r.get("payload");
                serde_json::from_value(payload)
            })
            .collect::<Result<_, _>>()
            .map_err(RepositoryError::Deserialization)?;

        let state =
            rehydrate(events).map_err(|e| RepositoryError::Rehydration(e.to_string()))?;

        Ok(Some(state))
    }

    async fn append_many(
        &self,
        match_report_id: &str,
        events: Vec<MatchReportDomainEvent>,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;
        let mut version = expected_version;
        for event in &events {
            version += 1;
            let payload = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;
            sqlx::query(
                "INSERT INTO match_report_event_store
                    (match_report_id, event_type, event_version, payload, version)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(match_report_id)
            .bind(event.type_name())
            .bind(event.schema_version())
            .bind(&payload)
            .bind(version as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(ref db) = e {
                    if db.constraint() == Some("match_report_es_version") {
                        return RepositoryError::ConcurrentWrite;
                    }
                }
                RepositoryError::Database(e)
            })?;
            self.update_projection_in_tx(&mut tx, match_report_id, event, version).await?;
        }
        tx.commit().await.map_err(RepositoryError::Database)?;
        Ok(version)
    }

    async fn find_id_by_pairing(
        &self,
        pairing_id: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query(
            "SELECT match_report_id FROM match_report_projection
             WHERE pairing_id = $1
             LIMIT 1",
        )
        .bind(pairing_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(row.map(|r| r.get("match_report_id")))
    }

    async fn find_id_by_round_and_teams(
        &self,
        round_id: &str,
        team_a: &str,
        team_b: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query(
            "SELECT match_report_id FROM match_report_projection
             WHERE round_id = $1
               AND phase != 'Cancelled'
               AND (
                 (home_team_id = $2 AND away_team_id = $3)
                 OR (home_team_id = $3 AND away_team_id = $2)
               )
             LIMIT 1",
        )
        .bind(round_id)
        .bind(team_a)
        .bind(team_b)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(row.map(|r| r.get("match_report_id")))
    }

    async fn find_actions_by_match_and_side(
        &self,
        match_report_id: &str,
        side: TeamSide,
    ) -> Result<Vec<MatchActionRow>, RepositoryError> {
        use crate::app::match_report::domain::value_objects::TeamSide;
        let side_str = match side { TeamSide::Home => "home", TeamSide::Away => "away" };
        let rows = sqlx::query(
            "SELECT action_id, turn_number, player_id, player_type,
                    action_json, player_display_name, player_position
             FROM match_report_actions
             WHERE match_report_id = $1 AND team_side = $2 AND NOT is_deleted
             ORDER BY turn_number ASC, recorded_at ASC",
        )
        .bind(match_report_id)
        .bind(side_str)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(rows.iter().map(|r| MatchActionRow {
            action_id: r.get("action_id"),
            turn_number: r.get("turn_number"),
            player_id: r.get("player_id"),
            player_type: r.get("player_type"),
            action_json: r.get("action_json"),
            player_display_name: r.get("player_display_name"),
            player_position: r.get("player_position"),
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::shared_kernel::common_types::{
        CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
    };
    use crate::app::shared_kernel::team::TeamId;

    async fn test_pool() -> Option<PgPool> {
        dotenvy::dotenv().ok();
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect(&url).await.ok()
    }

    fn test_created_event(mr_id: &str) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MatchReportCreated {
            match_report_id: MatchReportId::try_new(mr_id).unwrap(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
        }
    }

    #[tokio::test]
    async fn append_and_find_by_id() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = MatchReportRepository::new(pool.clone());
        let mr_id = format!("{}", MatchReportId::new());

        let event = test_created_event(&mr_id);
        let v1 = repo.append(&mr_id, &event, 0).await.unwrap();
        assert_eq!(v1, 1);

        let state = repo.find_by_id(&mr_id).await.unwrap().unwrap();
        assert!(matches!(state, MatchReportState::Draft(_)));
        if let MatchReportState::Draft(draft) = &state {
            assert_eq!(draft.version, 1);
        }

        let event2 = MatchReportDomainEvent::SelectionConfirmed {
            confirmed_by: CoachId::new(),
        };
        let v2 = repo.append(&mr_id, &event2, 1).await.unwrap();
        assert_eq!(v2, 2);

        let state = repo.find_by_id(&mr_id).await.unwrap().unwrap();
        assert!(matches!(state, MatchReportState::PreMatch(_)));

        sqlx::query("DELETE FROM match_report_event_store WHERE match_report_id = $1")
            .bind(&mr_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM match_report_projection WHERE match_report_id = $1")
            .bind(&mr_id)
            .execute(&pool)
            .await
            .ok();
    }

    #[tokio::test]
    async fn concurrent_write_is_rejected() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = MatchReportRepository::new(pool.clone());
        let mr_id = format!("{}", MatchReportId::new());

        let event = test_created_event(&mr_id);
        repo.append(&mr_id, &event, 0).await.unwrap();

        let event2 = MatchReportDomainEvent::SelectionConfirmed {
            confirmed_by: CoachId::new(),
        };
        let dup = repo.append(&mr_id, &event2, 0).await;
        assert!(matches!(dup, Err(RepositoryError::ConcurrentWrite)));

        sqlx::query("DELETE FROM match_report_event_store WHERE match_report_id = $1")
            .bind(&mr_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM match_report_projection WHERE match_report_id = $1")
            .bind(&mr_id)
            .execute(&pool)
            .await
            .ok();
    }
}
