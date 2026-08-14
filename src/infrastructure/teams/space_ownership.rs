//! Qui possède une équipe — projection **et** brouillons (cartes 320 et 321).
//!
//! # Un paramètre, deux sources
//!
//! `{team_id}` est revendiqué par deux BCs pour deux choses différentes :
//! `teams` y voit une équipe enrôlée, vivant dans `team_proj` ; `team_creation`
//! y voit un brouillon, vivant dans `team_drafts`. Un brouillon n'apparaît
//! dans la projection **qu'à sa soumission**.
//!
//! La carte 320 n'a enregistré que la projection. Résultat : les brouillons non
//! encore soumis rendaient `404`, et la création d'équipe était cassée — 47
//! brouillons concernés sur la base de développement, et une suite e2e qui
//! l'aurait attrapé si elle avait été lancée.
//!
//! Le résolveur consulte donc **les deux sources, dans l'ordre du plus
//! probable** : une requête sur un identifiant d'équipe vise presque toujours
//! une équipe existante ; la seconde requête ne part que pour un brouillon.
//!
//! # La leçon, plus large que ce fichier
//!
//! Un nom de paramètre n'appartient pas à un BC. Deux BCs peuvent le
//! revendiquer, et le premier résolveur enregistré déciderait pour les deux si
//! le middleware acceptait les doublons — ce qu'il refuse désormais.

use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::team_creation::ports::ITeamDraftRepository;
use crate::app::teams::ports::ITeamRepository;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamSpaceOwnership {
    equipes: Arc<dyn ITeamRepository>,
    brouillons: Arc<dyn ITeamDraftRepository>,
}

impl TeamSpaceOwnership {
    pub fn new(
        equipes: Arc<dyn ITeamRepository>,
        brouillons: Arc<dyn ITeamDraftRepository>,
    ) -> Self {
        Self {
            equipes,
            brouillons,
        }
    }
}

#[async_trait]
impl ISpaceOwnership for TeamSpaceOwnership {
    fn param(&self) -> &'static str {
        "team_id"
    }

    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        match self.equipes.find_space_id(id).await {
            Ok(Some(brut)) => return SpaceId::try_new(&brut).ok(),
            Ok(None) => {}
            Err(e) => {
                tracing::error!("space_ownership teams {id} : {e:?}");
                return None;
            }
        }

        // Pas une équipe : peut-être un brouillon en cours de création.
        let Ok(team_id) = crate::app::shared_kernel::bloodbowl::team::TeamId::try_new(id) else {
            return None;
        };
        match self.brouillons.find_space_id(&team_id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership drafts {id} : {e:?}");
                None
            }
        }
    }
}
