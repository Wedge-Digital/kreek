//! Crée le joueur qu'un coach vient d'acheter dans `teams`.
//!
//! Recruter est un fait de `teams` ; l'entité joueur vit ici. Sans ce listener,
//! le coach paie un joueur qui n'existe nulle part.
//!
//! `init(app_event_bus: …)` : c'est cette signature que l'axe 5 de
//! `check-arch` reconnaît comme listener cross-BC, dont la projection ne peut
//! pas partager la transaction d'un commit distant.

use crate::app::players::io::app_events::player_creation::{
    creer_joueur, nom_de_poste, prochain_maillot_libre, ListenerError,
};
use crate::app::players::ports::{IPlayerProjectionRepository, ISkillCatalogPort};
use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

/// Le maillot est attribué **ici**, pas par `teams`, qui n'a aucune raison de
/// connaître les numéros. Les recrutements d'un même lot sont traités
/// séquentiellement : chacun voit l'état laissé par le précédent, donc deux
/// joueurs ne peuvent pas réserver le même numéro.
async fn handle_player_recruited(
    team_id: &str,
    space_id: &str,
    player_id: &str,
    roster_line_id: &str,
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
        pool,
        catalog,
    )
    .await
}

pub fn init(
    app_event_bus: &EventBus,
    pool: PgPool,
    projections: Arc<dyn IPlayerProjectionRepository>,
    skill_catalog: Arc<dyn ISkillCatalogPort>,
) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<TeamsAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    // `TeamsAppEvent` a désormais plusieurs variantes : ce
                    // listener ne traite que le recrutement, le renvoi a le sien.
                    let TeamsAppEvent::PlayerRecruited {
                        team_id,
                        space_id,
                        player_id,
                        roster_line_id,
                        ..
                    } = app_event
                    else {
                        continue;
                    };
                    if let Err(e) = handle_player_recruited(
                        &team_id.to_string(),
                        &space_id.to_string(),
                        &player_id.to_string(),
                        &roster_line_id,
                        &pool,
                        projections.as_ref(),
                        skill_catalog.as_ref(),
                    )
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
