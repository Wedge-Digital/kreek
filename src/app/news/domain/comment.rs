use crate::app::shared_kernel::common_types::{ArticleId, CommentId, UserId};

#[derive(Debug, Clone)]
pub struct Comment {
    pub id:          CommentId,
    pub article_id:  ArticleId,
    pub author_id:   UserId,
    pub author_name: String,
    pub content:     String,
    pub created_at:  time::OffsetDateTime,
}
