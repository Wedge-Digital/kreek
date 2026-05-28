use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::news::domain::comment::Comment;
use crate::app::news::domain::comment_repository_port::{CommentRepositoryError, ICommentRepository};
use crate::app::shared_kernel::common_types::{ArticleId, CommentId, UserId};

fn db_err(e: impl std::fmt::Display) -> CommentRepositoryError {
    CommentRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id:          String,
    article_id:  String,
    author_id:   String,
    author_name: String,
    content:     String,
    created_at:  time::OffsetDateTime,
}

impl TryFrom<CommentRow> for Comment {
    type Error = CommentRepositoryError;

    fn try_from(row: CommentRow) -> Result<Self, Self::Error> {
        Ok(Comment {
            id:          CommentId::try_new(&row.id).map_err(|e| db_err(e))?,
            article_id:  ArticleId::try_new(&row.article_id).map_err(|e| db_err(e))?,
            author_id:   UserId::try_new(&row.author_id).map_err(|e| db_err(e))?,
            author_name: row.author_name,
            content:     row.content,
            created_at:  row.created_at,
        })
    }
}

#[derive(Clone)]
pub struct CommentRepository {
    pool: PgPool,
}

impl CommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ICommentRepository for CommentRepository {
    async fn save(&self, comment: &Comment) -> Result<(), CommentRepositoryError> {
        sqlx::query(include_str!("sql/comments/insert_comment.sql"))
            .bind(comment.id.to_string())
            .bind(comment.article_id.to_string())
            .bind(comment.author_id.to_string())
            .bind(&comment.content)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(())
    }

    async fn find_by_article(
        &self,
        article_id: &ArticleId,
    ) -> Result<Vec<Comment>, CommentRepositoryError> {
        let rows: Vec<CommentRow> = sqlx::query_as(
            include_str!("sql/comments/find_comments_by_article.sql")
        )
        .bind(article_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter().map(Comment::try_from).collect()
    }
}