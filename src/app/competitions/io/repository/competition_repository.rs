use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::app::shared_kernel::competition_profile::CompetitionProfile;

fn db_err(e: impl std::fmt::Display) -> CompetitionRepositoryError {
    CompetitionRepositoryError::Database(e.to_string())
}

#[derive(Clone)]
pub struct CompetitionRepository {
    pool: PgPool,
}

impl CompetitionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ICompetitionRepository for CompetitionRepository {
    async fn name_exists_in_space(&self, name: &CompetitionName, space_id: &SpaceId) -> Result<bool, CompetitionRepositoryError> {
        let exists: bool = sqlx::query_scalar(include_str!("sql/competitions/find_by_name_in_space.sql"))
            .bind(space_id.to_string())
            .bind(name.value())
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(exists)
    }

    async fn save(&self, competition: &Competition) -> Result<(), CompetitionRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(include_str!("sql/competitions/insert_competition.sql"))
            .bind(competition.id.to_string())
            .bind(competition.space_id.to_string())
            .bind(competition.name.value())
            .bind(competition.logo.as_ref())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for admin_id in &competition.admin_ids {
            sqlx::query(include_str!("sql/competitions/insert_competition_member.sql"))
                .bind(competition.id.to_string())
                .bind(admin_id.to_string())
                .bind(CompetitionProfile::CompetitionAdmin.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn find_by_space_id(&self, space_id: &SpaceId) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { id: String, name: String, logo: String, status: String }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/competitions/find_by_space_id.sql"))
            .bind(space_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| CompetitionSummary { id: r.id, name: r.name, logo: r.logo, status: r.status }).collect())
    }

    async fn find_rules(&self, competition_id: &CompetitionId) -> Result<Option<CompetitionRules>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { rules: Option<String> }

        let row: Option<Row> = sqlx::query_as::<_, Row>(include_str!("sql/competitions/select_rules.sql"))
            .bind(competition_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.rules) else { return Ok(None) };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| CompetitionRepositoryError::Database(e.to_string()))
    }

    async fn save_rules(&self, competition_id: &CompetitionId, rules: &CompetitionRules) -> Result<(), CompetitionRepositoryError> {
        let json = serde_json::to_string(rules)
            .map_err(|e| CompetitionRepositoryError::Database(e.to_string()))?;

        let found: Option<String> = sqlx::query_scalar(include_str!("sql/competitions/update_rules.sql"))
            .bind(json)
            .bind(competition_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(CompetitionRepositoryError::CompetitionNotFound);
        }

        Ok(())
    }
}