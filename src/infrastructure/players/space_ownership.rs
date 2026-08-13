//! `players` répond sur ses propres joueurs (carte 324).
//!
//! Vit dans `infrastructure/` et non dans le BC : c'est l'hôte qui compose le
//! middleware avec les BCs, et le trait `ISpaceOwnership` appartient à
//! `src/web/`. Un BC qui l'implémenterait chez lui dépendrait de la couche web
//! de l'hôte.

use crate::app::players::ports::IPlayerProjectionRepository;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct PlayerSpaceOwnership {
    projections: Arc<dyn IPlayerProjectionRepository>,
}

impl PlayerSpaceOwnership {
    pub fn new(projections: Arc<dyn IPlayerProjectionRepository>) -> Self {
        Self { projections }
    }
}

#[async_trait]
impl ISpaceOwnership for PlayerSpaceOwnership {
    fn param(&self) -> &'static str {
        "player_id"
    }

    /// Une erreur de base rend `None`, donc un `404`. Refuser sur incertitude
    /// est le seul comportement défendable pour un contrôle d'accès : laisser
    /// passer parce que la base a hoqueté ouvrirait la porte au moment précis
    /// où l'on est le moins capable de la surveiller.
    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        match self.projections.find_space_id(id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership players {id} : {e}");
                None
            }
        }
    }
}
