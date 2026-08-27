//! Retrait d'une customisation déjà appliquée au joueur.

use crate::app::players::domain::customisations::{flux_effectif, trouver, undo_pour};
use crate::app::players::domain::error::DomainError;
use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{Player, ValueKpo};
use crate::app::players::ports::{IPlayerRepository, RepositoryError};
use crate::app::players::use_cases::commands::RevertCustomisationCommand;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub enum RevertCustomisationError {
    PlayerNotFound,
    /// Aucune customisation de cet identifiant ne s'applique encore. Couvre les
    /// deux cas d'un seul mot : elle n'a jamais existé, ou elle a déjà été
    /// retirée — l'écran ne les distingue pas et n'aurait pas à le faire.
    CustomisationIntrouvable,
    Domain(DomainError),
    Repository(RepositoryError),
}

impl From<RepositoryError> for RevertCustomisationError {
    fn from(e: RepositoryError) -> Self {
        Self::Repository(e)
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RevertCustomisationCommand,
    player_repo: &dyn IPlayerRepository,
    event_bus: &EventBus,
) -> Result<(), RevertCustomisationError> {
    let player = player_repo
        .find_by_id(&cmd.player_id)
        .await?
        .ok_or(RevertCustomisationError::PlayerNotFound)?;
    let events = player_repo.find_events_by_id(&cmd.player_id).await?;

    let visee = trouver(&events, &cmd.customisation_id)
        .ok_or(RevertCustomisationError::CustomisationIntrouvable)?;
    let undo = undo_pour(visee, valeur_sans(&events, &cmd))
        .ok_or(RevertCustomisationError::CustomisationIntrouvable)?;

    let event = player
        .revert_customisation(cmd.customisation_id, undo, cmd.author)
        .map_err(RevertCustomisationError::Domain)?;

    player_repo
        .append(&player.id, &player.team_id, &event, player.version + 1)
        .await?;
    emettre(event_bus, event.to_enveloppe(&player.id.0));
    Ok(())
}

/// La valeur qu'aurait le joueur si cette customisation n'avait jamais eu lieu.
///
/// **Exacte par construction** : c'est le joueur rejoué sans elle, pas un calcul
/// inverse. L'écrêtage à zéro de `apply` n'étant pas inversible, il n'y a pas
/// d'autre moyen — un joueur à 30 kPo qui subit un −50 tombe à 0, et lui rendre
/// +50 lui donnerait 50.
///
/// Le rejeu porte sur le **flux effectif** et non sur le flux privé du seul
/// événement visé : voir `customisations::flux_effectif`, dont c'est toute la
/// raison d'être.
///
/// La valeur nulle du repli n'est jamais atteinte — `trouver` a déjà prouvé que
/// le flux contient un `PlayerCreated`, sans quoi il n'aurait rien rendu.
fn valeur_sans(events: &[PlayerDomainEvent], cmd: &RevertCustomisationCommand) -> ValueKpo {
    let effectif: Vec<PlayerDomainEvent> = flux_effectif(events, Some(&cmd.customisation_id))
        .into_iter()
        .cloned()
        .collect();
    Player::from_events(&effectif)
        .map(|p| p.value)
        .unwrap_or(ValueKpo(0))
}
