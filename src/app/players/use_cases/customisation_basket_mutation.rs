//! Les cinq mutations du panier de customisation.
//!
//! Elles ont la même forme et **ne décident de rien** : hydrater, appeler la
//! méthode domaine, persister sous garde de version. Bornes, doublons, plancher
//! de prix et plafond de SPP sont évalués par l'agrégat, jamais ici.
//!
//! **Aucune ne rend l'agrégat muté.** Ce serait un cadeau empoisonné : `save`
//! rend la nouvelle version sans la reposer sur l'agrégat, dont le champ
//! `version` reste celui d'avant écriture. Un appelant qui le cuirait dans les
//! `hx-vals` du prochain geste ferait échouer chaque second clic en écriture
//! concurrente — le piège de la carte 264, qui ne se voit qu'en navigateur.
//! Les handlers relisent, et c'est la seule façon correcte.

use crate::app::players::domain::customisation_basket::CustomisationBasket;
use crate::app::players::domain::error::DomainError;
use crate::app::players::ports::{
    CustomisationBasketState, ICustomisationBasketRepository, IPlayerRepository, ISkillCatalogPort,
    RepositoryError,
};
use crate::app::players::use_cases::commands::{
    AddCustomisationSkillCommand, AddCustomisationSppCommand, AddCustomisationStatCommand,
    AdjustCustomisationPriceCommand, RemoveCustomisationLineCommand,
};
use crate::app::players::use_cases::customisation_basket_hydration_service::{
    hydrate, HydrationError,
};

#[derive(Debug)]
pub enum CustomisationBasketError {
    PlayerNotFound,
    /// Un autre onglet a modifié le panier entre son affichage et ce geste.
    ConcurrentWrite,
    Domain(DomainError),
    Hydration(HydrationError),
    Repository(RepositoryError),
}

impl From<RepositoryError> for CustomisationBasketError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::ConcurrentWrite,
            autre => Self::Repository(autre),
        }
    }
}

impl From<HydrationError> for CustomisationBasketError {
    fn from(e: HydrationError) -> Self {
        match e {
            HydrationError::PlayerNotFound => Self::PlayerNotFound,
            autre => Self::Hydration(autre),
        }
    }
}

pub async fn add_skill(
    cmd: AddCustomisationSkillCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError> {
    let (mut basket, _) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;
    basket
        .add_skill(cmd.skill_id)
        .map_err(CustomisationBasketError::Domain)?;
    persister(&basket, space_id, basket_repo, cmd.expected_version).await
}

pub async fn add_stat(
    cmd: AddCustomisationStatCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError> {
    let (mut basket, _) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;
    basket
        .add_stat(cmd.stat, cmd.crans)
        .map_err(CustomisationBasketError::Domain)?;
    persister(&basket, space_id, basket_repo, cmd.expected_version).await
}

pub async fn adjust_price(
    cmd: AdjustCustomisationPriceCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError> {
    let (mut basket, _) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;
    basket
        .adjust_price(cmd.delta)
        .map_err(CustomisationBasketError::Domain)?;
    persister(&basket, space_id, basket_repo, cmd.expected_version).await
}

pub async fn add_spp(
    cmd: AddCustomisationSppCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError> {
    let (mut basket, _) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;
    basket
        .add_spp(cmd.amount)
        .map_err(CustomisationBasketError::Domain)?;
    persister(&basket, space_id, basket_repo, cmd.expected_version).await
}

pub async fn remove_line(
    cmd: RemoveCustomisationLineCommand,
    space_id: &str,
    player_repo: &dyn IPlayerRepository,
    basket_repo: &dyn ICustomisationBasketRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), CustomisationBasketError> {
    let (mut basket, _) = hydrate(&cmd.player_id, player_repo, basket_repo, catalog).await?;
    basket
        .remove_line(&cmd.line_id)
        .map_err(CustomisationBasketError::Domain)?;
    persister(&basket, space_id, basket_repo, cmd.expected_version).await
}

/// Persiste les **lignes seules** — le reste est rechargé à chaque hydratation.
async fn persister(
    basket: &CustomisationBasket,
    space_id: &str,
    basket_repo: &dyn ICustomisationBasketRepository,
    expected_version: u32,
) -> Result<(), CustomisationBasketError> {
    let state = serde_json::to_value(basket.lines())
        .map_err(|e| CustomisationBasketError::Repository(RepositoryError::Serialization(e)))?;

    basket_repo
        .save(
            &CustomisationBasketState {
                player_id: basket.player_id().0.clone(),
                space_id: space_id.to_string(),
                state,
                version: basket.version().0,
                // Ignoré à l'écriture : c'est la base qui pose `updated_at`,
                // et c'est ce qui fait glisser la fenêtre de péremption.
                updated_at: time::OffsetDateTime::UNIX_EPOCH,
            },
            expected_version,
        )
        .await?;
    Ok(())
}
