use async_trait::async_trait;
use sqlx::{PgPool, Row};
use crate::app::competitions::domain::cache_repository_port::{
    CachedSpace, CachedUser, CompetitionsCacheError, ICompetitionsCacheRepository,
};
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CoachId, SpaceId};
use crate::app::shared_kernel::email::Email;

#[derive(sqlx::FromRow)]
struct UserRow {
    id:         String,
    coach_name: String,
    coach_icon: Option<String>,
    email:      String,
}

impl UserRow {
    fn into_cached_user(self) -> CachedUser {
        CachedUser {
            id:         CoachId::try_new(&self.id).unwrap(),
            coach_name: CoachName::try_new(self.coach_name).unwrap(),
            coach_icon: self.coach_icon.map(|s| CoachIcon::try_new(s).unwrap()),
            email:      Email::try_new(self.email).unwrap(),
        }
    }
}

fn db_err(e: impl std::fmt::Display) -> CompetitionsCacheError {
    CompetitionsCacheError::Database(e.to_string())
}

#[derive(Clone)]
pub struct CompetitionsCacheRepository {
    pool: PgPool,
}

impl CompetitionsCacheRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ICompetitionsCacheRepository for CompetitionsCacheRepository {
    async fn add_user(&self, user: &CachedUser) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/insert_user.sql"))
            .bind(user.id.to_string())
            .bind(user.coach_name.clone().into_inner())
            .bind(user.coach_icon.as_ref().map(|i| i.as_ref()))
            .bind(user.email.value())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn remove_user(&self, id: &CoachId) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/delete_user.sql"))
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn add_space(&self, space: &CachedSpace) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/insert_space.sql"))
            .bind(space.space_id.to_string())
            .bind(&space.space_name.to_string())
            .bind(&space.space_logo.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn remove_space(&self, space_id: &SpaceId) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/delete_space.sql"))
            .bind(space_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn subscribe(&self, coach_id: &CoachId, space_id: &SpaceId, profile: &SpaceProfile) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/subscribe_user.sql"))
            .bind(space_id.to_string())
            .bind(coach_id.to_string())
            .bind(profile.as_str())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn unsubscribe(&self, coach_id: &CoachId, space_id: &SpaceId) -> Result<(), CompetitionsCacheError> {
        sqlx::query(include_str!("sql/cache/unsubscribe_user.sql"))
            .bind(coach_id.to_string())
            .bind(space_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn list_members_for_space(&self, space_id: &SpaceId) -> Result<Vec<CachedUser>, CompetitionsCacheError> {
        let rows = sqlx::query_as::<_, UserRow>(include_str!("sql/cache/list_members_for_space.sql"))
            .bind(space_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows.into_iter().map(UserRow::into_cached_user).collect())
    }
}