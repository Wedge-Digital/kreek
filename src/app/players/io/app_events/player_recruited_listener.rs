//! Crée le joueur que `teams` vient d'ajouter à un effectif.
//!
//! Recruter est un fait de `teams` ; l'entité joueur vit ici. Sans ce listener,
//! le coach paie un joueur qui n'existe nulle part.
//!
//! **Deux naissances, un seul chemin.** Un joueur acheté naît `Active`, un
//! journalier aligné naît `Journeyman` — même corps, même attribution de
//! maillot, seul le statut diffère. Les séparer en deux listeners aurait
//! dupliqué la seule chose qui compte ici : que le numéro soit attribué en
//! voyant ceux déjà pris.
//!
//! Le désalignement d'un journalier passe **aussi** par ici, parce qu'il est
//! le pendant exact de sa naissance : le coach a refait sa composition, et le
//! journalier qui en sort n'a jamais été embauché.
//!
//! `init(app_event_bus: …)` : c'est cette signature que l'axe 5 de
//! `check-arch` reconnaît comme listener cross-BC, dont la projection ne peut
//! pas partager la transaction d'un commit distant.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{PlayerId, RosterMembership, TeamId};
use crate::app::players::io::app_events::player_creation::{
    creer_joueur, nom_de_poste, prochain_maillot_libre, ListenerError,
};
use crate::app::players::io::repository::player_repository::{
    insert_player_event, upsert_player_projection,
};
use crate::app::players::ports::{
    IPlayerProjectionRepository, IPlayerRepository, ISkillCatalogPort,
};
use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

/// Le maillot est attribué **ici**, pas par `teams`, qui n'a aucune raison de
/// connaître les numéros. Les recrutements d'un même lot sont traités
/// séquentiellement : chacun voit l'état laissé par le précédent, donc deux
/// joueurs ne peuvent pas réserver le même numéro.
#[allow(clippy::too_many_arguments)]
async fn handle_player_recruited(
    team_id: &str,
    space_id: &str,
    player_id: &str,
    roster_line_id: &str,
    membership: RosterMembership,
    pool: &PgPool,
    projections: &dyn IPlayerProjectionRepository,
    catalog: &dyn ISkillCatalogPort,
) -> Result<(), ListenerError> {
    let jersey = prochain_maillot_libre(team_id, projections).await;
    creer_joueur(
        team_id,
        space_id,
        player_id,
        roster_line_id,
        &nom_de_poste(roster_line_id, catalog),
        jersey,
        membership,
        pool,
        catalog,
    )
    .await
}

/// Sort le journalier de l'effectif — il n'y avait jamais été embauché.
///
/// **Par l'agrégat et non par un `UPDATE`** : le désalignement est un fait, il
/// doit laisser sa trace dans l'event store. Un joueur introuvable n'est pas
/// une anomalie — le retrait peut arriver avant que sa naissance ait été
/// traitée, ou après un redémarrage.
async fn retirer_journalier(
    player_id: &str,
    team_id: &str,
    pool: &PgPool,
    repository: &dyn IPlayerRepository,
) {
    let id = PlayerId(player_id.to_string());
    let joueur = match repository.find_by_id(&id).await {
        Ok(Some(j)) => j,
        Ok(None) => {
            tracing::debug!(
                "players player_recruited_listener: journalier {player_id} inconnu, rien à retirer"
            );
            return;
        }
        Err(e) => {
            tracing::error!("players player_recruited_listener: lecture {player_id}: {e}");
            return;
        }
    };
    let event = PlayerDomainEvent::JourneymanWithdrawn {
        player_id: id,
        team_id: TeamId(team_id.to_string()),
    };
    if let Err(e) = appliquer(pool, &event, joueur.version + 1).await {
        tracing::error!("players player_recruited_listener: retrait {player_id}: {e}");
    }
}

/// L'événement et sa projection dans **une seule transaction**, comme toute
/// écriture event-sourcée de ce BC.
async fn appliquer(
    pool: &PgPool,
    event: &PlayerDomainEvent,
    version: i32,
) -> Result<(), ListenerError> {
    let mut tx = pool.begin().await.map_err(ListenerError::Database)?;
    insert_player_event(&mut tx, event, version).await?;
    upsert_player_projection(&mut tx, event).await?;
    tx.commit().await.map_err(ListenerError::Database)?;
    Ok(())
}

pub fn init(
    app_event_bus: &EventBus,
    pool: PgPool,
    projections: Arc<dyn IPlayerProjectionRepository>,
    skill_catalog: Arc<dyn ISkillCatalogPort>,
    repository: Arc<dyn IPlayerRepository>,
) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<TeamsAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    // `TeamsAppEvent` a désormais plusieurs variantes : ce
                    // listener traite les deux naissances et le désalignement,
                    // le renvoi a le sien.
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    if let TeamsAppEvent::JourneymanWithdrawn {
                        team_id, player_id, ..
                    } = &app_event
                    {
                        retirer_journalier(
                            &player_id.to_string(),
                            &team_id.to_string(),
                            &pool,
                            repository.as_ref(),
                        )
                        .instrument(span)
                        .await;
                        continue;
                    }
                    let (team_id, space_id, player_id, roster_line_id, membership) = match app_event
                    {
                        TeamsAppEvent::PlayerRecruited {
                            team_id,
                            space_id,
                            player_id,
                            roster_line_id,
                            ..
                        } => (
                            team_id,
                            space_id,
                            player_id,
                            roster_line_id,
                            RosterMembership::Active,
                        ),
                        TeamsAppEvent::JourneymanFielded {
                            team_id,
                            space_id,
                            player_id,
                            roster_line_id,
                            ..
                        } => (
                            team_id,
                            space_id,
                            player_id,
                            roster_line_id,
                            RosterMembership::Journeyman,
                        ),
                        _ => continue,
                    };
                    if let Err(e) = handle_player_recruited(
                        &team_id.to_string(),
                        &space_id.to_string(),
                        &player_id.to_string(),
                        &roster_line_id,
                        membership,
                        &pool,
                        projections.as_ref(),
                        skill_catalog.as_ref(),
                    )
                    .instrument(span)
                    .await
                    {
                        match e {
                            ListenerError::AlreadyProcessed => tracing::warn!(
                                "players player_recruited_listener: joueur {player_id} déjà créé"
                            ),
                            other => tracing::error!(
                                "players player_recruited_listener: {other} (équipe {team_id})"
                            ),
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("players player_recruited_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
