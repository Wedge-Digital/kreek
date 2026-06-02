use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::user::User;
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    coach_name: String,
    coach_icon: Option<String>,
    email: String,
    password_hash: String,
}

impl TryFrom<UserRow> for User {
    type Error = RepositoryError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User::new(
            UserId::try_new(&row.id).map_err(db_err)?,
            CoachName::try_new(row.coach_name).map_err(db_err)?,
            row.coach_icon
                .map(CoachIcon::try_new)
                .transpose()
                .map_err(db_err)?,
            Email::try_new(row.email).map_err(db_err)?,
            row.password_hash,
        ))
    }
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        UserRepository { pool }
    }
}

#[async_trait]
impl IUserRepository for UserRepository {
    async fn create(&self, user: &User) -> Result<(), RepositoryError> {
        sqlx::query(include_str!("sql/insert_user.sql"))
            .bind(user.id.to_string())
            .bind(user.coach_name.clone().into_inner())
            .bind(user.coach_icon.as_ref().map(|i| i.as_ref()))
            .bind(user.email.value())
            .bind(&user.password_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        let constraint = db_err.constraint().unwrap_or("");
                        return if constraint.contains("coach_name") {
                            RepositoryError::CoachNameAlreadyTaken
                        } else {
                            RepositoryError::EmailAlreadyTaken
                        };
                    }
                }
                RepositoryError::Database(e.to_string())
            })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(include_str!("sql/find_user_by_id.sql"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(row.map(User::try_from).transpose()?)
    }

    async fn find_by_coach_name(&self, coach_name: &str) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(include_str!("sql/find_user_by_coach_name.sql"))
            .bind(coach_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(row.map(User::try_from).transpose()?)
    }

    async fn update_password_hash(
        &self,
        coach_name: &str,
        new_hash: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(include_str!("sql/update_user_password.sql"))
            .bind(new_hash)
            .bind(coach_name)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}
