use crate::app::news::domain::article_tag::ArticleTag;
use crate::app::news::domain::article_title::ArticleTitle;
use crate::app::news::domain::paragraph_type::ParagraphType;
use crate::app::shared_kernel::common_types::{ArticleId, SpaceId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleParagraph {
    #[serde(rename = "type")]
    pub paragraph_type: ParagraphType,
    pub title: String,   // arch:ok texte libre
    pub content: String, // arch:ok texte libre
}

#[derive(Debug, Clone)]
pub struct Article {
    pub id: ArticleId,
    pub space_id: SpaceId,
    pub author_id: UserId,
    pub author_name: String,  // arch:ok texte libre dénormalisé
    pub title: ArticleTitle,
    pub abstract_: String,    // arch:ok texte libre
    pub tags: Vec<ArticleTag>,
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
        title: ArticleTitle,
        abstract_: String,
        tags: Vec<ArticleTag>,
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
