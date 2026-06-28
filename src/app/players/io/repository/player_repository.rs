use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{AcquisitionMode, Player, PlayerId, TeamId};
use crate::app::players::ports::{AcquiredSkillProjection, IPlayerRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

pub struct PgPlayerRepository {
    pool: PgPool,
}

impl PgPlayerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn event_type_name(event: &PlayerDomainEvent) -> &'static str {
    match event {
        PlayerDomainEvent::PlayerCreated { .. } => "PlayerCreated",
        PlayerDomainEvent::InitialSkillEarned   { .. } => "InitialSkillEarned",
    }
}

fn player_and_team_id(event: &PlayerDomainEvent) -> (&str, &str) {
    match event {
        PlayerDomainEvent::PlayerCreated { player_id, team_id, .. } => (&player_id.0, &team_id.0),
        PlayerDomainEvent::InitialSkillEarned   { player_id, team_id, .. } => (&player_id.0, &team_id.0),
    }
}

// ── Fonctions transactionnelles ───────────────────────────────────────────────

pub async fn insert_player_event(
    tx:      &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event:   &PlayerDomainEvent,
    version: i32,
) -> Result<(), RepositoryError> {
    let (player_id, team_id) = player_and_team_id(event);
    let payload = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;

    sqlx::query(
        "INSERT INTO players_events (player_id, team_id, event_type, payload, version)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(player_id)
    .bind(team_id)
    .bind(event_type_name(event))
    .bind(&payload)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db) = e {
            if db.constraint() == Some("players_events_player_version") {
                return RepositoryError::ConcurrentWrite;
            }
        }
        RepositoryError::Database(e)
    })?;

    Ok(())
}

pub async fn upsert_player_projection(
    tx:    &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &PlayerDomainEvent,
) -> Result<(), RepositoryError> {
    match event {
        PlayerDomainEvent::PlayerCreated {
            player_id, team_id, space_id, position_name, roster_line_id,
            jersey, base_skills, starting_spp, starting_value,
        } => {
            let skill_ids: Vec<&str> = base_skills.iter().map(|s| s.as_ref()).collect();
            let base_json = serde_json::to_value(&skill_ids).map_err(RepositoryError::Serialization)?;

            sqlx::query(
                "INSERT INTO players_projection
                     (player_id, team_id, space_id, position_name, roster_line_id,
                      personal_name, jersey, base_skills, acquired_skills, spp, value_kpo, version)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, '[]'::jsonb, $9, $10, 1)
                 ON CONFLICT (player_id) DO NOTHING",
            )
            .bind(&player_id.0)
            .bind(&team_id.0)
            .bind(space_id.to_string())
            .bind(position_name.as_ref())
            .bind(roster_line_id.as_ref())
            .bind("")
            .bind(jersey.map(|j| j.into_inner() as i16))
            .bind(&base_json)
            .bind(starting_spp.0 as i32)
            .bind(starting_value.0 as i32)
            .execute(&mut **tx)
            .await
            .map_err(RepositoryError::Database)?;
        }

        PlayerDomainEvent::InitialSkillEarned {
            player_id, skill_id, skill_name, category_css, mode, spp_cost, value_delta, ..
        } => {
            let acq = AcquiredSkillProjection {
                skill_id:     skill_id.as_ref().to_string(),
                skill_name:   skill_name.as_ref().to_string(),
                category_css: category_css.clone(),
                mode: match mode {
                    AcquisitionMode::Chosen => "Chosen".to_string(),
                    AcquisitionMode::Random => "Random".to_string(),
                },
                spp_cost: spp_cost.into_inner() as i32,
            };
            let acq_json = serde_json::to_value(&acq).map_err(RepositoryError::Serialization)?;

            sqlx::query(
                "UPDATE players_projection
                 SET acquired_skills = acquired_skills || jsonb_build_array($1::jsonb),
                     value_kpo = value_kpo + $2,
                     version   = version + 1
                 WHERE player_id = $3",
            )
            .bind(&acq_json)
            .bind(value_delta.0 as i32)
            .bind(&player_id.0)
            .execute(&mut **tx)
            .await
            .map_err(RepositoryError::Database)?;
        }
    }

    Ok(())
}

#[async_trait]
impl IPlayerRepository for PgPlayerRepository {
    async fn append(
        &self,
        _player_id: &PlayerId,
        _team_id:   &TeamId,
        event:      &PlayerDomainEvent,
        version:    i32,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;
        insert_player_event(&mut tx, event, version).await?;
        upsert_player_projection(&mut tx, event).await?;
        tx.commit().await.map_err(RepositoryError::Database)?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        player_id: &PlayerId,
    ) -> Result<Option<Player>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT payload FROM players_events
             WHERE player_id = $1 ORDER BY version ASC",
        )
        .bind(&player_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let events: Vec<PlayerDomainEvent> = rows
            .iter()
            .map(|r| serde_json::from_value(r.get::<serde_json::Value, _>("payload")))
            .collect::<Result<_, _>>()
            .map_err(RepositoryError::Deserialization)?;

        Ok(Player::from_events(&events))
    }

    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<Player>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT player_id, payload FROM players_events
             WHERE team_id = $1 ORDER BY player_id, version ASC",
        )
        .bind(&team_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        let mut groups: HashMap<String, Vec<PlayerDomainEvent>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();

        for row in &rows {
            let pid: String = row.get("player_id");
            let event: PlayerDomainEvent =
                serde_json::from_value(row.get::<serde_json::Value, _>("payload"))
                    .map_err(RepositoryError::Deserialization)?;

            if !groups.contains_key(&pid) {
                order.push(pid.clone());
            }
            groups.entry(pid).or_default().push(event);
        }

        let players = order
            .iter()
            .filter_map(|pid| Player::from_events(groups.get(pid)?))
            .collect();

        Ok(players)
    }
}
