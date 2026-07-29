use crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent;
use crate::app::teams::ports::{
    IJourneymanTypePort, IPlayerValuePort, IRosterInfoPort, ITeamRepository,
};
use crate::app::teams::use_cases::recompute_team_value_use_case;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Recalcule la TV quand `players` annonce que le roster initial est complet.
///
/// Sans lui, une équipe créée avec `auto_enroll` — le cas normal dès que la
/// saison ne demande pas de validation — atteint `ReadyToPlay` pendant que
/// `players` insère encore ses joueurs, et se fige à une TV de zéro.
///
/// Listener **cross-BC** : la signature `init(app_event_bus: ...)` est ce qui
/// le distingue d'un listener intra-BC pour l'axe 5 de `check-arch`, et ce qui
/// l'exempte de la règle de transaction unique — l'événement vient d'un commit
/// déjà passé dans un autre BC.
///
/// Aucun ordre n'est à garantir : `TeamValueRecomputed` porte une valeur
/// absolue, donc si `TeamEnrolled` a déjà déclenché un recalcul prématuré, ce
/// second append l'écrase. Et tout recalcul postérieur à cet événement voit un
/// roster complet, par construction.
pub fn init(
    app_event_bus: &EventBus,
    repo: Arc<dyn ITeamRepository>,
    player_value_port: Arc<dyn IPlayerValuePort>,
    roster_info_port: Arc<dyn IRosterInfoPort>,
    journeyman_type_port: Arc<dyn IJourneymanTypePort>,
) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(PlayersAppEvent::InitialRosterCompleted { team_id, .. }) =
                        serde_json::from_value::<PlayersAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    if let Err(e) = recompute_team_value_use_case::execute(
                        &team_id,
                        repo.as_ref(),
                        player_value_port.as_ref(),
                        roster_info_port.as_ref(),
                        journeyman_type_port.as_ref(),
                    )
                    .await
                    {
                        tracing::error!(
                            "initial_roster_listener: recalcul de la TV de {team_id} : {e:?}"
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("initial_roster_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
