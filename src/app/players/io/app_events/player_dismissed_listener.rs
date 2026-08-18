//! Sort de l'effectif le joueur que le coach vient de renvoyer dans `teams`.
//!
//! Renvoyer est une décision de `teams` ; l'appartenance à l'effectif est un
//! fait de `players`. Sans ce listener, le renvoi n'a aucun effet réel : le
//! joueur reste compté, dans la valeur d'équipe comme dans les journaliers.
//!
//! `init(app_event_bus: …)` : c'est cette signature que l'axe 5 de
//! `check-arch` reconnaît comme listener cross-BC, dont la projection ne peut
//! pas partager la transaction d'un commit distant.
//!
//! Il réémet ensuite le fait sur le **bus interne** de `players`, ce qui le fera
//! ressortir en app event. Ce n'est pas un accusé de réception : c'est
//! `players` qui énonce un fait de son propre domaine, et c'est le seul instant
//! où l'effectif à jour est lisible. `teams` s'en sert pour recalculer sa valeur
//! d'équipe — même mécanique qu'`InitialRosterCompleted` à la création.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{PlayerId, TeamId};
use crate::app::players::ports::{IPlayerRepository, RepositoryError};
use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;
use tracing::Instrument;

#[derive(Debug)]
enum DismissalError {
    /// Le joueur a déjà quitté l'effectif — un app event reçu deux fois, ou
    /// rejoué. Ce n'est pas une anomalie, seulement un travail déjà fait.
    DejaSorti,
    Introuvable,
    Repository(RepositoryError),
}

/// Charge l'agrégat, appende le renvoi à la version suivante, et retourne
/// l'événement pour que l'appelant le publie **après** l'écriture.
///
/// L'idempotence tient à la lecture de `membership` plutôt qu'à la seule
/// contrainte d'unicité : un second app event trouverait un joueur déjà sorti
/// et n'écrirait rien, au lieu d'échouer sur une version prise.
async fn sortir_de_l_effectif(
    player_id: &str,
    team_id: &str,
    repo: &dyn IPlayerRepository,
) -> Result<PlayerDomainEvent, DismissalError> {
    let id = PlayerId(player_id.to_string());
    let joueur = repo
        .find_by_id(&id)
        .await
        .map_err(DismissalError::Repository)?
        .ok_or(DismissalError::Introuvable)?;

    if !joueur.membership.is_active() {
        return Err(DismissalError::DejaSorti);
    }

    let event = PlayerDomainEvent::PlayerDismissed {
        player_id: id.clone(),
        team_id: TeamId(team_id.to_string()),
    };
    repo.append(
        &id,
        &TeamId(team_id.to_string()),
        &event,
        joueur.version + 1,
    )
    .await
    .map_err(DismissalError::Repository)?;

    Ok(event)
}

pub fn init(app_event_bus: &EventBus, event_bus: EventBus, repo: Arc<dyn IPlayerRepository>) {
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
                    let TeamsAppEvent::PlayerDismissed {
                        team_id, player_id, ..
                    } = app_event
                    else {
                        continue;
                    };

                    let team = team_id.to_string();
                    let joueur = player_id.to_string();
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    match sortir_de_l_effectif(&joueur, &team, repo.as_ref())
                        .instrument(span)
                        .await
                    {
                        // Émis **après** l'écriture : c'est ce qui garantit
                        // qu'un recalcul de valeur d'équipe déclenché par cet
                        // événement voit un effectif à jour.
                        Ok(event) => {
                            let _ = event_bus.send(event.to_enveloppe(&joueur));
                        }
                        Err(DismissalError::DejaSorti) => tracing::warn!(
                            "players player_dismissed_listener: joueur {joueur} déjà sorti"
                        ),
                        Err(e) => tracing::error!(
                            "players player_dismissed_listener: {e:?} (équipe {team})"
                        ),
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("players player_dismissed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
