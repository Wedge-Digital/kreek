use crate::app::news::domain::article::{Article, ArticleParagraph};
use crate::app::news::domain::article_repository_port::{
    ArticleRepositoryError, IArticleRepository,
};
use crate::app::news::domain::article_tag::ArticleTag;
use crate::app::news::domain::article_title::ArticleTitle;
use crate::app::shared_kernel::bloodbowl::ids::ArticleId;
use crate::app::shared_kernel::identity::ids::{SpaceId, UserId};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: impl std::fmt::Display) -> ArticleRepositoryError {
    ArticleRepositoryError::Database(e.to_string())
}

#[derive(sqlx::FromRow)]
struct ArticleRow {
    id: String,
    space_id: String,
    author_id: String,
    author_name: String,
    title: String,
    #[sqlx(rename = "abstract")]
    abstract_: String,
    tags: Vec<String>,
    image: Option<String>,
    content: serde_json::Value,
    created_at: time::OffsetDateTime,
}

impl TryFrom<ArticleRow> for Article {
    type Error = ArticleRepositoryError;

    fn try_from(row: ArticleRow) -> Result<Self, Self::Error> {
        let paragraphs: Vec<ArticleParagraph> =
            serde_json::from_value(row.content).map_err(|e| db_err(e))?;
        let title = ArticleTitle::try_new(row.title).map_err(|e| db_err(e))?;
        let tags = row
            .tags
            .iter()
            .filter_map(|s| s.parse::<ArticleTag>().ok())
            .collect();
        Ok(Article::new(
            ArticleId::try_new(&row.id).map_err(|e| db_err(e))?,
            SpaceId::try_new(&row.space_id).map_err(|e| db_err(e))?,
            UserId::try_new(&row.author_id).map_err(|e| db_err(e))?,
            row.author_name,
            title,
            row.abstract_,
            tags,
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
        let content_json = serde_json::to_value(&article.content).map_err(|e| db_err(e))?;

        let tag_strings: Vec<String> = article.tags.iter().map(|t| t.to_string()).collect();
        sqlx::query(include_str!("sql/articles/insert_article.sql"))
            .bind(article.id.to_string())
            .bind(article.space_id.to_string())
            .bind(article.author_id.to_string())
            .bind(article.title.as_ref())
            .bind(&article.abstract_)
            .bind(&tag_strings)
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
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Article>, i64), ArticleRepositoryError> {
        let offset = (page - 1) * per_page;

        let rows: Vec<ArticleRow> =
            sqlx::query_as(include_str!("sql/articles/find_articles_by_space.sql"))
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

        let articles = rows
            .into_iter()
            .map(Article::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((articles, total))
    }

    async fn find_space_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<String>, ArticleRepositoryError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT space_id FROM articles WHERE id = $1")
            .bind(article_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ArticleRepositoryError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn find_by_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<Article>, ArticleRepositoryError> {
        let row: Option<ArticleRow> =
            sqlx::query_as(include_str!("sql/articles/find_article_by_id.sql"))
                .bind(article_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        row.map(Article::try_from).transpose()
    }
}
