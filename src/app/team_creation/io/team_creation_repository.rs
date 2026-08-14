use crate::app::shared_kernel::bloodbowl::team::{BaseTeamInfo, TeamId, TeamName};
use crate::app::shared_kernel::identity::ids::Entity;
use crate::app::shared_kernel::identity::ids::{CoachId, UserId};
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{
    ITeamDraftRepository, ITeamRosterRepository, RepositoryError,
};
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Clone)]
pub struct TeamDraftRepository {
    pool: PgPool,
}

impl TeamDraftRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db_err(e: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::PersistenceError(e.to_string())
}

#[async_trait]
impl ITeamDraftRepository for TeamDraftRepository {
    async fn save(&self, team: &DraftTeam, space_id: &str) -> Result<(), RepositoryError> {
        let logo = team.base_infos().logo_url().map(|u| u.as_ref().to_string());
        let creation_rules = serde_json::to_value(team.creation_rules())
            .map_err(|e| RepositoryError::PersistenceError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO team_drafts (id, space_id, competition_id, season_id, name, coach_id, coach_name, logo, creation_rules)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO UPDATE SET
               name           = EXCLUDED.name,
               coach_id       = EXCLUDED.coach_id,
               coach_name     = EXCLUDED.coach_name,
               logo           = EXCLUDED.logo,
               creation_rules = EXCLUDED.creation_rules",
        )
        .bind(team.get_id().to_string())
        .bind(space_id)
        .bind(team.competition_id())
        .bind(team.season_id())
        .bind(team.base_infos().name().clone().into_inner())
        .bind(team.base_infos().coach_id().to_string())
        .bind(team.coach_name())
        .bind(logo)
        .bind(creation_rules)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn find_space_id(&self, id: &TeamId) -> Result<Option<String>, RepositoryError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT space_id FROM team_drafts WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(row.map(|r| r.0))
    }

    async fn find_by_id(&self, id: &TeamId) -> Result<Option<DraftTeam>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            created_by: String,
            competition_id: String,
            season_id: String,
            name: String,
            coach_id: String,
            coach_name: String,
            logo: Option<String>,
            creation_rules: serde_json::Value,
        }

        let row: Option<Row> = sqlx::query_as(
            "SELECT id, coach_id AS created_by, competition_id, season_id,
                    name, coach_id, coach_name, logo, creation_rules
             FROM team_drafts WHERE id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        let Some(r) = row else { return Ok(None) };

        let entity_id = TeamId::try_new(&r.id).map_err(db_err)?;
        let created_by = UserId::try_new(&r.created_by).map_err(db_err)?;
        let coach_id = CoachId::try_new(&r.coach_id).map_err(db_err)?;
        let team_name = TeamName::try_new(r.name).map_err(db_err)?;
        let logo = r.logo.as_deref().and_then(|u| {
            crate::app::shared_kernel::identity::ids::CloudinaryImage::try_new(u).ok()
        });
        let creation_rules: CreationRules = serde_json::from_value(r.creation_rules)
            .map_err(|e| RepositoryError::PersistenceError(e.to_string()))?;

        let base_infos = BaseTeamInfo::new(team_name, coach_id, logo);
        Ok(Some(DraftTeam::from_parts(
            entity_id,
            created_by,
            base_infos,
            r.coach_name,
            r.competition_id,
            r.season_id,
            creation_rules,
        )))
    }

    async fn find_by_coach_and_space(
        &self,
        coach_id: &str,
        space_id: &str,
    ) -> Result<Vec<DraftTeam>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            created_by: String,
            competition_id: String,
            season_id: String,
            name: String,
            coach_id: String,
            coach_name: String,
            logo: Option<String>,
            creation_rules: serde_json::Value,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, coach_id AS created_by, competition_id, season_id,
                    name, coach_id, coach_name, logo, creation_rules
             FROM team_drafts
             WHERE coach_id = $1 AND space_id = $2
             ORDER BY created_at DESC",
        )
        .bind(coach_id)
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter()
            .map(|r| {
                let entity_id = TeamId::try_new(&r.id).map_err(db_err)?;
                let created_by = UserId::try_new(&r.created_by).map_err(db_err)?;
                let coach_id_val = CoachId::try_new(&r.coach_id).map_err(db_err)?;
                let team_name = TeamName::try_new(r.name).map_err(db_err)?;
                let logo = r.logo.as_deref().and_then(|u| {
                    crate::app::shared_kernel::identity::ids::CloudinaryImage::try_new(u).ok()
                });
                let creation_rules: CreationRules = serde_json::from_value(r.creation_rules)
                    .map_err(|e| RepositoryError::PersistenceError(e.to_string()))?;
                let base_infos = BaseTeamInfo::new(team_name, coach_id_val, logo);
                Ok(DraftTeam::from_parts(
                    entity_id,
                    created_by,
                    base_infos,
                    r.coach_name,
                    r.competition_id,
                    r.season_id,
                    creation_rules,
                ))
            })
            .collect()
    }
}

// ── TeamRosterRepository ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TeamRosterRepository {
    pool: PgPool,
}

impl TeamRosterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ITeamRosterRepository for TeamRosterRepository {
    async fn save(&self, team: &RosterSelectedTeam, space_id: &str) -> Result<(), RepositoryError> {
        let state = serde_json::to_value(team)
            .map_err(|e| RepositoryError::PersistenceError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO team_roster_selections (id, space_id, state, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (id) DO UPDATE SET
               state      = EXCLUDED.state,
               updated_at = now()",
        )
        .bind(team.get_id().to_string())
        .bind(space_id)
        .bind(state)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TeamId) -> Result<Option<RosterSelectedTeam>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            state: serde_json::Value,
        }

        let row: Option<Row> =
            sqlx::query_as("SELECT state FROM team_roster_selections WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(r) = row else { return Ok(None) };

        let team: RosterSelectedTeam = serde_json::from_value(r.state)
            .map_err(|e| RepositoryError::PersistenceError(e.to_string()))?;

        Ok(Some(team))
    }

    async fn mark_submitted(&self, id: &TeamId) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE team_roster_selections SET submitted_at = now() WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn find_submitted_ids_for_space(
        &self,
        space_id: &str,
    ) -> Result<Vec<String>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id FROM team_roster_selections WHERE space_id = $1 AND submitted_at IS NOT NULL",
        )
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| r.id).collect())
    }
}
