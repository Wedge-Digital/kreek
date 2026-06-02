use crate::app::shared_kernel::common_types::{ArticleId, SpaceId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleParagraph {
    #[serde(rename = "type")]
    pub paragraph_type: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Article {
    pub id: ArticleId,
    pub space_id: SpaceId,
    pub author_id: UserId,
    pub author_name: String,
    pub title: String,
    pub abstract_: String,
    pub tags: Vec<String>,
    pub image: Option<String>,
    pub content: Vec<ArticleParagraph>,
    pub created_at: time::OffsetDateTime,
}

impl Article {
    pub fn new(
        id: ArticleId,
        space_id: SpaceId,
        author_id: UserId,
        author_name: String,
        title: String,
        abstract_: String,
        tags: Vec<String>,
        image: Option<String>,
        content: Vec<ArticleParagraph>,
        created_at: time::OffsetDateTime,
    ) -> Self {
        Self {
            id,
            space_id,
            author_id,
            author_name,
            title,
            abstract_,
            tags,
            image,
            content,
            created_at,
        }
    }
}
