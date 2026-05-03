use crate::app::auth::domain::coach_name::CoachName;
use crate::app::auth::domain::email::Email;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::user::User;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

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
        sqlx::query(
            "INSERT INTO users (id, coach_name, email, password_hash) VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id.to_string())
        .bind(user.coach_name.value())
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

    async fn find_by_coach_name(&self, coach_name: &str) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, coach_name, email, password_hash FROM users WHERE coach_name = $1",
        )
        .bind(coach_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let id_str: String         = row.try_get("id")            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let coach_name_str: String = row.try_get("coach_name")    .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let email_str: String      = row.try_get("email")         .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let password_hash: String  = row.try_get("password_hash") .map_err(|e| RepositoryError::Database(e.to_string()))?;

        let id = UserId::from_string(&id_str)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let coach_name = CoachName::new(coach_name_str)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        let email = Email::new(email_str)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(Some(User::new(id, coach_name, email, password_hash)))
    }
}
