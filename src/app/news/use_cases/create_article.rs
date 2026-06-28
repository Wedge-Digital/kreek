use crate::app::news::domain::article::{Article, ArticleParagraph};
use crate::app::news::domain::article_repository_port::{
    ArticleRepositoryError, IArticleRepository,
};
use crate::app::news::domain::article_tag::ArticleTag;
use crate::app::news::domain::article_title::ArticleTitle;
use crate::app::shared_kernel::common_types::{ArticleId, SpaceId, UserId};

#[derive(Debug)]
pub enum CreateArticleError {
    EmptyTitle,
    EmptyContent,
    Database(String),
}

impl std::fmt::Display for CreateArticleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateArticleError::EmptyTitle => write!(f, "Le titre est obligatoire"),
            CreateArticleError::EmptyContent => {
                write!(f, "L'article doit contenir au moins un paragraphe")
            }
            CreateArticleError::Database(m) => write!(f, "Erreur interne : {}", m),
        }
    }
}

impl From<ArticleRepositoryError> for CreateArticleError {
    fn from(e: ArticleRepositoryError) -> Self {
        match e {
            ArticleRepositoryError::Database(m) => CreateArticleError::Database(m),
        }
    }
}

pub struct CreateArticleCommand {
    pub space_id: SpaceId,
    pub author_id: UserId,
    pub author_name: String,
    pub title: ArticleTitle,
    pub abstract_: String,
    pub tags: Vec<ArticleTag>,
    pub image: Option<String>,
    pub paragraphs: Vec<ArticleParagraph>,
}

pub async fn execute(
    cmd: CreateArticleCommand,
    repo: &dyn IArticleRepository,
) -> Result<ArticleId, CreateArticleError> {
    if cmd.paragraphs.is_empty() {
        return Err(CreateArticleError::EmptyContent);
    }

    let id = ArticleId::new();
    let article = Article::new(
        id,
        cmd.space_id,
        cmd.author_id,
        cmd.author_name,
        cmd.title,
        cmd.abstract_.trim().to_string(),
        cmd.tags,
        cmd.image.filter(|s| !s.trim().is_empty()),
        cmd.paragraphs,
        time::OffsetDateTime::now_utc(),
    );

    repo.save(&article).await?;

    Ok(article.id)
}
