use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId, UserId};
use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::shared_kernel::email::Email;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::{ISpaceUserCacheRepository, SpaceUserCacheRepositoryError};
use crate::app::spaces::domain::user::User;

fn db_err(e: impl std::fmt::Display) -> SpaceUserCacheRepositoryError {
    SpaceUserCacheRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct Row {
    coach_id:        String,
    coach_name:      String,
    coach_icon: String,
    email:           String,
}

#[derive(Clone)]
pub struct SpaceUserCacheRepository {
    pool: PgPool,
}

impl SpaceUserCacheRepository {
    pub fn new(pool: PgPool) -> Self {
        SpaceUserCacheRepository { pool }
    }
}

#[async_trait]
impl ISpaceUserCacheRepository for SpaceUserCacheRepository {
    async fn save(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError> {
        sqlx::query(include_str!("sql/user_cache/insert_user.sql"))
            .bind(user.id.to_string())
            .bind(user.name.clone().into_inner())
            .bind(user.icon.to_string())
            .bind(user.email.as_ref().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        return SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache;
                    }
                }
                SpaceUserCacheRepositoryError::Database(e.to_string())
            })?;
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
        let rows = sqlx::query_as::<_, Row>(include_str!("sql/user_cache/find_all_users.sql"))
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: UserId::try_new(&r.coach_id).unwrap(),
                name: CoachName::try_new(r.coach_name).unwrap(),
                icon: CoachIcon::try_new(r.coach_icon).unwrap(),
                email: Email::try_new(r.email).unwrap(),
                })
            .collect())
    }

    async fn find_by_id(&self, coach_id: &CoachId) -> Result<Option<User>, SpaceUserCacheRepositoryError> {

        let rows = sqlx::query_as::<_, Row>(include_str!("sql/user_cache/find_user_by_id.sql"))
            .bind(coach_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .next()
            .map(|r| User {
                id:    UserId::try_new(&r.coach_id).unwrap(),
                name:  CoachName::try_new(r.coach_name).unwrap(),
                icon:  CoachIcon::try_new(r.coach_icon).unwrap(),
                email: Email::try_new(r.email).unwrap(),
            }))
    }

}