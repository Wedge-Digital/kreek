use crate::app::news::domain::comment::Comment;
use crate::app::shared_kernel::bloodbowl::ids::ArticleId;
use async_trait::async_trait;

#[derive(Debug)]
pub enum CommentRepositoryError {
    Database(String),
}

impl std::fmt::Display for CommentRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommentRepositoryError::Database(msg) => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

#[async_trait]
pub trait ICommentRepository: Send + Sync {
    async fn save(&self, comment: &Comment) -> Result<(), CommentRepositoryError>;

    async fn find_by_article(
        &self,
        article_id: &ArticleId,
    ) -> Result<Vec<Comment>, CommentRepositoryError>;
}
