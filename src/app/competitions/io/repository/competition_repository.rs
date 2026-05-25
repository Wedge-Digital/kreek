use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, SpaceId};
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
        struct Row { id: String, name: String, logo: String, season_id: Option<String>, status: Option<String> }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/competitions/find_by_space_id.sql"))
            .bind(space_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| CompetitionSummary {
            id:        r.id,
            name:      r.name,
            logo:      r.logo,
            season_id: r.season_id,
            status:    r.status,
        }).collect())
    }

    async fn find_base_info(&self, competition_id: &CompetitionId) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { competition_name: String, logo: Option<String>, coach_id: Option<String>, coach_name: Option<String> }

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/competitions/find_competition_by_id.sql"))
            .bind(competition_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        if rows.is_empty() { return Ok(None); }

        let name        = rows[0].competition_name.clone();
        let logo        = rows[0].logo.clone();
        let admin_ids   = rows.iter().filter_map(|r| r.coach_id.clone()).collect();
        let admin_names = rows.into_iter().filter_map(|r| r.coach_name).collect();

        Ok(Some(CompetitionBaseInfo { name, logo, admin_ids, admin_names }))
    }

    async fn update_base_info(&self, competition_id: &CompetitionId, name: &CompetitionName, logo: &CloudinaryImage, admin_ids: &[CoachId]) -> Result<(), CompetitionRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        let found: Option<String> = sqlx::query_scalar(include_str!("sql/competitions/update_base_info.sql"))
            .bind(name.value())
            .bind(logo.as_ref())
            .bind(competition_id.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(CompetitionRepositoryError::CompetitionNotFound);
        }

        sqlx::query("DELETE FROM competitions_members WHERE competition_id = $1")
            .bind(competition_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for admin_id in admin_ids {
            sqlx::query(include_str!("sql/competitions/insert_competition_member.sql"))
                .bind(competition_id.to_string())
                .bind(admin_id.to_string())
                .bind(CompetitionProfile::CompetitionAdmin.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
}