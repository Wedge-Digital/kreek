use crate::app::auth::domain::reset_token::{ResetToken, Token};
use crate::app::auth::ports::RepositoryError;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Database(e.to_string())
}

#[async_trait]
pub trait IResetTokenRepository: Send + Sync {
    async fn find_by_token(&self, token: &str) -> Result<Option<ResetToken>, RepositoryError>;
    async fn create(&self, token: &Token, coach_name: &CoachName) -> Result<(), RepositoryError>;
    async fn delete_by_token(&self, token: &str) -> Result<(), RepositoryError>;
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    token: String,
    coach_name: String,
    created_at: time::OffsetDateTime,
}

impl TryFrom<TokenRow> for ResetToken {
    type Error = RepositoryError;

    fn try_from(row: TokenRow) -> Result<Self, Self::Error> {
        Ok(ResetToken {
            token: Token::try_new(&row.token).map_err(db_err)?,
            coach_name: CoachName::try_new(row.coach_name).map_err(db_err)?,
            created_at: row.created_at,
        })
    }
}

#[derive(Clone)]
pub struct ResetTokenRepository {
    pool: PgPool,
}

impl ResetTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        ResetTokenRepository { pool }
    }
}

#[async_trait]
impl IResetTokenRepository for ResetTokenRepository {
    async fn find_by_token(&self, token: &str) -> Result<Option<ResetToken>, RepositoryError> {
        let row =
            sqlx::query_as::<_, TokenRow>(include_str!("sql/find_token_infos_by_token_value.sql"))
                .bind(token)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(row.map(ResetToken::try_from).transpose()?)
    }

    async fn create(&self, token: &Token, coach_name: &CoachName) -> Result<(), RepositoryError> {
        sqlx::query(include_str!("sql/insert_reset_token.sql"))
            .bind(token.to_string())
            .bind(coach_name.clone().into_inner())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(())
    }

    async fn delete_by_token(&self, token: &str) -> Result<(), RepositoryError> {
        sqlx::query(include_str!("sql/delete_reset_token.sql"))
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}
