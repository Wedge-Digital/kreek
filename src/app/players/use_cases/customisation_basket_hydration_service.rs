//! Hydratation du panier de customisation.
//!
//! Seul point où les DTOs des ports deviennent du domaine : les handlers ne
//! voient jamais un `SkillCatalogEntryDto`, ils reçoivent l'agrégat hydraté.
//!
//! Les lignes viennent du panier persisté ; **tout le reste est rechargé** —
//! caractéristiques résolues, compétences possédées, catalogue, valeur et SPP.
//! C'est ce qui garantit qu'un panier d'une heure est jugé contre le joueur
//! d'aujourd'hui, et non contre celui de sa création.

use crate::app::players::domain::customisation_basket::{
    BasketVersion, CustomisationBasket, CustomisationLine, ResolvedStats,
};
use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::players::domain::value_objects::SkillId;
use crate::app::players::ports::{
    ICustomisationBasketRepository, IPlayerRepository, ISkillCatalogPort, RepositoryError,
};
use crate::app::players::use_cases::player_stats_service;

#[derive(Debug)]
pub enum HydrationError {
    PlayerNotFound,
    /// Le poste du joueur est introuvable au catalogue : ses caractéristiques
    /// ne peuvent pas être résolues, donc aucune borne ne peut être jugée.
    UnknownPosition,
    /// Les lignes persistées ne se relisent pas — panier écrit par une version
    /// antérieure du format, ou corrompu.
    CorruptedBasket(serde_json::Error),
    Repository(RepositoryError),
}

impl From<RepositoryError> for HydrationError {
    fn from(e: RepositoryError) -> Self {
        Self::Repository(e)
    }
}

/// Charge et hydrate le panier d'un joueur. **Un panier absent donne un panier
/// vide en version zéro** — c'est ainsi que le premier geste du commissaire
/// l'ouvre, sans endpoint dédié.
pub async fn hydrate(
    player_id: &PlayerId,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(CustomisationBasket, Player), HydrationError> {
    let player = player_repo
        .find_by_id(player_id)
        .await?
        .ok_or(HydrationError::PlayerNotFound)?;

    let stats = player_stats_service::resolve_stats(&player, catalog)
        .ok_or(HydrationError::UnknownPosition)?;

    let persiste = basket_repo.load(&player_id.0).await?;
    let (version, lines) = match persiste {
        Some(etat) => {
            let lignes: Vec<CustomisationLine> =
                serde_json::from_value(etat.state).map_err(HydrationError::CorruptedBasket)?;
            (BasketVersion(etat.version), lignes)
        }
        None => (BasketVersion(0), Vec::new()),
    };

    let basket = CustomisationBasket::hydrate(
        player_id.clone(),
        version,
        lines,
        ResolvedStats {
            ma: stats.ma,
            st: stats.st,
            ag: stats.ag,
            pa: stats.pa,
            av: stats.av,
        },
        competences_possedees(&player),
        catalog
            .list_all_skills()
            .into_iter()
            .filter_map(|s| SkillId::try_new(s.skill_id).ok())
            .collect(),
        player.value,
        player.spp,
    );

    Ok((basket, player))
}

/// Compétences de base **et** acquises — quelle qu'en soit l'origine. La règle
/// de non-doublon ne distingue pas : une compétence obtenue en SPP bloque son
/// ajout par customisation, et réciproquement.
fn competences_possedees(player: &Player) -> Vec<SkillId> {
    player
        .base_skills
        .iter()
        .cloned()
        .chain(player.acquired_skills.iter().map(|a| a.skill_id.clone()))
        .collect()
}
