use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::news::domain::article::{Article, ArticleParagraph};
use crate::app::news::domain::article_repository_port::{ArticleRepositoryError, IArticleRepository};
use crate::app::shared_kernel::common_types::{ArticleId, SpaceId, UserId};

fn db_err(e: impl std::fmt::Display) -> ArticleRepositoryError {
    ArticleRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct ArticleRow {
    id:          String,
    space_id:    String,
    author_id:   String,
    author_name: String,
    title:       String,
    #[sqlx(rename = "abstract")]
    abstract_:   String,
    tags:        Vec<String>,
    image:       Option<String>,
    content:     serde_json::Value,
    created_at:  time::OffsetDateTime,
}

impl TryFrom<ArticleRow> for Article {
    type Error = ArticleRepositoryError;

    fn try_from(row: ArticleRow) -> Result<Self, Self::Error> {
        let paragraphs: Vec<ArticleParagraph> = serde_json::from_value(row.content)
            .map_err(|e| db_err(e))?;
        Ok(Article::new(
            ArticleId::try_new(&row.id).map_err(|e| db_err(e))?,
            SpaceId::try_new(&row.space_id).map_err(|e| db_err(e))?,
            UserId::try_new(&row.author_id).map_err(|e| db_err(e))?,
            row.author_name,
            row.title,
            row.abstract_,
            row.tags,
            row.image,
            paragraphs,
            row.created_at,
        ))
    }
}

#[derive(Clone)]
pub struct ArticleRepository {
    pool: PgPool,
}

impl ArticleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IArticleRepository for ArticleRepository {
    async fn save(&self, article: &Article) -> Result<(), ArticleRepositoryError> {
        let content_json = serde_json::to_value(&article.content)
            .map_err(|e| db_err(e))?;

        sqlx::query(include_str!("sql/articles/insert_article.sql"))
            .bind(article.id.to_string())
            .bind(article.space_id.to_string())
            .bind(article.author_id.to_string())
            .bind(&article.title)
            .bind(&article.abstract_)
            .bind(&article.tags)
            .bind(article.image.as_deref())
            .bind(content_json)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(())
    }

    async fn find_by_space(
        &self,
        space_id: &SpaceId,
        page:     i64,
        per_page: i64,
    ) -> Result<(Vec<Article>, i64), ArticleRepositoryError> {
        let offset = (page - 1) * per_page;

        let rows: Vec<ArticleRow> = sqlx::query_as(
            include_str!("sql/articles/find_articles_by_space.sql")
        )
        .bind(space_id.to_string())
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE space_id = $1")
            .bind(space_id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;

        let articles = rows.into_iter()
            .map(Article::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((articles, total))
    }

    async fn find_by_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<Article>, ArticleRepositoryError> {
        let row: Option<ArticleRow> = sqlx::query_as(
            include_str!("sql/articles/find_article_by_id.sql")
        )
        .bind(article_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        row.map(Article::try_from).transpose()
    }
}