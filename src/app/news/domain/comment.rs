use crate::app::shared_kernel::common_types::{ArticleId, CommentId, UserId};

#[derive(Debug, Clone)]
pub struct Comment {
    pub id: CommentId,
    pub article_id: ArticleId,
    pub author_id: UserId,
    pub author_name: String, // arch:ok texte libre dénormalisé
    pub content: String,     // arch:ok texte libre
    pub created_at: time::OffsetDateTime,
}
