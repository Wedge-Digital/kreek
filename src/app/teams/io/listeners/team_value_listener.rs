use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::ports::{
    IJourneymanTypePort, IRosterCatalogPort, ISquadPort, ITeamRepository,
};
use crate::app::teams::use_cases::recompute_team_value_use_case;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Les quatre événements dont `apply()` pose `game_phase = ReadyToPlay`.
/// L'équipe est alors dans l'état où sa valeur doit refléter son effectif réel —
/// c'est le seul moment où la TV bouge.
fn ends_in_ready_to_play(event: &TeamDomainEvent) -> bool {
    matches!(
        event,
        TeamDomainEvent::TeamEnrolled { .. }
            | TeamDomainEvent::DismissalsPhaseValidated
            | TeamDomainEvent::MatchReportingCancelled { .. }
            | TeamDomainEvent::CostlyMistakesApplied { .. }
    )
}

/// Listener **intra-BC** : il écoute le bus interne de `teams`, alimenté par
/// `TeamRepository::append`. La signature `init(event_bus: ...)` est la
/// convention que `check-arch` (axe 5) utilise pour le distinguer d'un listener
/// cross-BC — ne pas la renommer sans lire cet axe.
pub fn init(
    event_bus: &EventBus,
    repo: Arc<dyn ITeamRepository>,
    squad_port: Arc<dyn ISquadPort>,
    roster_catalog_port: Arc<dyn IRosterCatalogPort>,
    journeyman_type_port: Arc<dyn IJourneymanTypePort>,
) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<TeamDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    // `TeamValueRecomputed` n'est surtout pas un déclencheur :
                    // le recalcul appende, l'append publie, et le listener
                    // recevrait son propre événement — boucle infinie.
                    if !ends_in_ready_to_play(&event) {
                        continue;
                    }
                    let team_id = envelope.emitter.clone();
                    if let Err(e) = recompute_team_value_use_case::execute(
                        &team_id,
                        repo.as_ref(),
                        squad_port.as_ref(),
                        roster_catalog_port.as_ref(),
                        journeyman_type_port.as_ref(),
                    )
                    .await
                    {
                        tracing::error!("team_value_listener: recalcul de {team_id} : {e:?}");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("team_value_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, SeasonId};
    use crate::app::teams::domain::value_objects::{IncidentType, Kpo};

    #[test]
    fn les_quatre_entrees_en_ready_to_play_declenchent_le_recalcul() {
        assert!(ends_in_ready_to_play(&TeamDomainEvent::TeamEnrolled {
            competition_id: CompetitionId::new(),
            competition_name: "C".into(),
            season_id: SeasonId::new(),
            season_name: "S".into(),
        }));
        assert!(ends_in_ready_to_play(
            &TeamDomainEvent::DismissalsPhaseValidated
        ));
        assert!(ends_in_ready_to_play(
            &TeamDomainEvent::MatchReportingCancelled {
                match_report_id: MatchReportId::new(),
            }
        ));
        assert!(ends_in_ready_to_play(
            &TeamDomainEvent::CostlyMistakesApplied {
                roll: 3,
                incident: IncidentType::None,
                gp_lost: Kpo(0),
            }
        ));
    }

    /// Sans cette exclusion, le recalcul appende, l'append publie, et le
    /// listener se rappelle lui-même sans fin.
    #[test]
    fn team_value_recomputed_ne_se_declenche_pas_lui_meme() {
        assert!(!ends_in_ready_to_play(
            &TeamDomainEvent::TeamValueRecomputed { value: Kpo(550) }
        ));
    }

    #[test]
    fn les_autres_evenements_ne_declenchent_rien() {
        assert!(!ends_in_ready_to_play(
            &TeamDomainEvent::RecruitmentPhaseValidated
        ));
        assert!(!ends_in_ready_to_play(&TeamDomainEvent::TeamDismissed));
    }
}
