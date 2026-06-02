use crate::app::news::domain::article_repository_port::IArticleRepository;
use crate::app::news::domain::comment_repository_port::ICommentRepository;
use crate::app::news::io::repository::article_repository::ArticleRepository;
use crate::app::news::io::repository::comment_repository::CommentRepository;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct NewsContext {
    pub article_repository: Arc<dyn IArticleRepository>,
    pub comment_repository: Arc<dyn ICommentRepository>,
}

impl NewsContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            article_repository: Arc::new(ArticleRepository::new(pool.clone())),
            comment_repository: Arc::new(CommentRepository::new(pool.clone())),
        }
    }
}
