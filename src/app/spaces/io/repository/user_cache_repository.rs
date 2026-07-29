use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
    ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
};
use crate::app::spaces::domain::user::User;
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> SpaceUserCacheRepositoryError {
    SpaceUserCacheRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct Row {
    #[sqlx(rename = "id")]
    coach_id: String,
    coach_name: String,
    coach_icon: Option<String>,
    email: String,
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
    async fn add_user(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError> {
        sqlx::query(include_str!("sql/user_cache/insert_user.sql"))
            .bind(user.id.to_string())
            .bind(user.name.clone().into_inner())
            .bind(user.email.as_ref().to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        return match db_err.constraint() {
                            Some("space_user_cache_coach_name_uq") => {
                                SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache
                            }
                            other => SpaceUserCacheRepositoryError::Database(format!(
                                "unique constraint violation on {:?}: {e}",
                                other
                            )),
                        };
                    }
                }
                SpaceUserCacheRepositoryError::Database(e.to_string())
            })?;
        Ok(())
    }

    async fn find_all_users(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
        let rows = sqlx::query_as::<_, Row>(include_str!("sql/user_cache/find_all_users.sql"))
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: CoachId::try_new(&r.coach_id).unwrap(),
                name: CoachName::try_new(r.coach_name).unwrap(),
                icon: r.coach_icon.map(|s| CoachIcon::try_new(s).unwrap()),
                email: Email::try_new(r.email).unwrap(),
            })
            .collect())
    }

    async fn find_user_by_id(
        &self,
        coach_id: &CoachId,
    ) -> Result<User, SpaceUserCacheRepositoryError> {
        tracing::debug!("find_user_by_id: {}", coach_id);
        let rows = sqlx::query_as::<_, Row>(include_str!("sql/user_cache/find_user_by_id.sql"))
            .bind(coach_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

        rows.into_iter()
            .next()
            .map(|r| User {
                id: CoachId::try_new(&r.coach_id).unwrap(),
                name: CoachName::try_new(r.coach_name).unwrap(),
                icon: r.coach_icon.map(|s| CoachIcon::try_new(s).unwrap()),
                email: Email::try_new(r.email).unwrap(),
            })
            .ok_or(SpaceUserCacheRepositoryError::UserNotFoundInCache)
    }

    async fn list_members_for_space(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
        let rows =
            sqlx::query_as::<_, Row>(include_str!("sql/user_cache/list_members_for_space.sql"))
                .bind(space_id.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: CoachId::try_new(&r.coach_id).unwrap(),
                name: CoachName::try_new(&r.coach_name).expect(&format!(
                    "CoachName invalide en base : '{}'",
                    r.coach_name.clone()
                )),
                icon: r.coach_icon.map(|s| CoachIcon::try_new(s).unwrap()),
                email: Email::try_new(r.email).unwrap(),
            })
            .collect())
    }
}
