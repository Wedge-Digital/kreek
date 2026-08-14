//! `news` répond sur ses articles (carte 322).
//!
//! **Un seul résolveur, contrairement à ce que la carte annonçait.** Elle
//! prévoyait un saut `comments` → `articles`, par symétrie avec les saisons.
//! Il est inutile : aucune route de ce BC ne porte d'identifiant de
//! commentaire. Les commentaires s'atteignent par
//! `/home/articles/{article_id}/comments`, donc contrôler l'article suffit.
//!
//! C'est exactement le cas prouvé exploitable par l'audit : un commentaire
//! avait été posté sur un article d'un autre espace.

use crate::app::news::domain::article_repository_port::IArticleRepository;
use crate::app::shared_kernel::bloodbowl::ids::ArticleId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ArticleSpaceOwnership {
    repo: Arc<dyn IArticleRepository>,
}

impl ArticleSpaceOwnership {
    pub fn new(repo: Arc<dyn IArticleRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ISpaceOwnership for ArticleSpaceOwnership {
    fn param(&self) -> &'static str {
        "article_id"
    }

    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        let article_id = ArticleId::try_new(id).ok()?;
        match self.repo.find_space_id(&article_id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership news {id} : {e:?}");
                None
            }
        }
    }
}
