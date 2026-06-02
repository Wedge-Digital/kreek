use async_trait::async_trait;
use sqlx::{PgPool, Row};
use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::ports::{ITeamRepository, RepositoryError};

pub struct TeamRepository {
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stub — implémenté en carte 42 (projection).
    async fn update_projection_in_tx(
        &self,
        _tx:      &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _team_id: &str,
        _event:   &TeamDomainEvent,
        _version: u64,
    ) -> Result<(), RepositoryError> {
        Ok(())
    }
}

#[async_trait]
impl ITeamRepository for TeamRepository {
    async fn append(
        &self,
        team_id:          &str,
        event:            &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let new_version   = expected_version + 1;
        let payload       = serde_json::to_value(event)
            .map_err(RepositoryError::Serialization)?;
        let event_type    = event.type_name();
        let event_version = event.schema_version();

        let mut tx = self.pool.begin().await
            .map_err(RepositoryError::Database)?;

        sqlx::query(
            "INSERT INTO team_event_store (team_id, event_type, event_version, payload, version)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(team_id)
        .bind(event_type)
        .bind(event_version)
        .bind(&payload)
        .bind(new_version as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db) = e {
                if db.constraint() == Some("team_event_store_version") {
                    return RepositoryError::ConcurrentWrite;
                }
            }
            RepositoryError::Database(e)
        })?;

        self.update_projection_in_tx(&mut tx, team_id, event, new_version).await?;

        tx.commit().await.map_err(RepositoryError::Database)?;
        Ok(new_version)
    }

    async fn find_by_id(&self, team_id: &str) -> Result<Option<Team>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT payload FROM team_event_store
             WHERE team_id = $1 ORDER BY version ASC",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let events: Vec<TeamDomainEvent> = rows.iter()
            .map(|r| {
                let payload: serde_json::Value = r.get("payload");
                serde_json::from_value(payload)
            })
            .collect::<Result<_, _>>()
            .map_err(RepositoryError::Deserialization)?;

        Ok(Team::hydrate(&events))
    }
}

// ── Tests d'intégration ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::value_objects::{Kpo, MatchResult};
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
    }

    fn created_event(team_id: &str) -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id:     team_id.to_string(),
            space_id:    "01SPACE00000000000000000000".to_string(),
            name:        "Les Korrigans FC".to_string(),
            roster_id:   "01ROST000000000000000000000".to_string(),
            roster_name: "Elfes Sylvestres".to_string(),
            coach_id:    "01COACH00000000000000000000".to_string(),
            coach_name:  "Colonel Castor".to_string(),
            treasury:    Kpo(1000),
        }
    }

    #[tokio::test]
    #[ignore = "nécessite make migrate pour créer team_event_store"]
    async fn append_and_find_by_id() {
        let Some(pool) = test_pool().await else { return };
        let repo    = TeamRepository::new(pool);
        let team_id = format!("TEST{}", ulid::Ulid::new());

        let event1 = created_event(&team_id);
        let v1 = repo.append(&team_id, &event1, 0).await.unwrap();
        assert_eq!(v1, 1);

        let event2 = TeamDomainEvent::TeamEnrolled {
            competition_id:   "01COMP000000000000000000000".to_string(),
            competition_name: "Ligue de Condate".to_string(),
            season_id:        "01SEAS000000000000000000000".to_string(),
            season_name:      "Saison 2025".to_string(),
        };
        let v2 = repo.append(&team_id, &event2, 1).await.unwrap();
        assert_eq!(v2, 2);

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        assert_eq!(team.name, "Les Korrigans FC");
        assert_eq!(team.version, 2);
        assert_eq!(
            team.participation_status,
            crate::app::teams::domain::team::ParticipationStatus::Enrolled,
        );

        // Cleanup
        sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
    }

    #[tokio::test]
    #[ignore = "nécessite make migrate pour créer team_event_store"]
    async fn concurrent_write_is_rejected() {
        let Some(pool) = test_pool().await else { return };
        let repo    = TeamRepository::new(pool);
        let team_id = format!("TEST{}", ulid::Ulid::new());

        let event = created_event(&team_id);
        repo.append(&team_id, &event, 0).await.unwrap();

        // Deuxième append avec la même expected_version → ConcurrentWrite
        let event2 = TeamDomainEvent::TeamDismissed;
        let result = repo.append(&team_id, &event2, 0).await;
        assert!(matches!(result, Err(RepositoryError::ConcurrentWrite)));

        // Cleanup
        sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
    }
}
