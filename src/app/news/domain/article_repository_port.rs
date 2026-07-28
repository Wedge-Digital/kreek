use crate::app::news::domain::article::Article;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::bloodbowl::ids::ArticleId;
use async_trait::async_trait;

#[derive(Debug)]
pub enum ArticleRepositoryError {
    Database(String),
}

impl std::fmt::Display for ArticleRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArticleRepositoryError::Database(msg) => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

#[async_trait]
pub trait IArticleRepository: Send + Sync {
    async fn save(&self, article: &Article) -> Result<(), ArticleRepositoryError>;

    async fn find_by_space(
        &self,
        space_id: &SpaceId,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Article>, i64), ArticleRepositoryError>;

    async fn find_by_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<Article>, ArticleRepositoryError>;
}
