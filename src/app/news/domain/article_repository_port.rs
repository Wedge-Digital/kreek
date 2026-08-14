use crate::app::news::domain::article::Article;
use crate::app::shared_kernel::bloodbowl::ids::ArticleId;
use crate::app::shared_kernel::identity::ids::SpaceId;
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

    /// L'espace auquel appartient cet article, ou `None` s'il n'existe pas
    /// (carte 324).
    ///
    /// Une seule colonne plutôt que l'article entier : le contrôle s'exécute
    /// sur chaque requête, et il n'a besoin que de ça.
    async fn find_space_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<String>, ArticleRepositoryError>;

    async fn find_by_id(
        &self,
        article_id: &ArticleId,
    ) -> Result<Option<Article>, ArticleRepositoryError>;
}
