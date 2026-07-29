use crate::app::teams::domain::team::{GamePhase, TeamDomainEvent};
use crate::app::teams::ports::IPhaseDraftRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Les quatre événements dont `apply()` pose `game_phase = ReadyToPlay`.
///
/// Volontairement dupliqué avec `team_value_listener` : les deux réagissent aux
/// mêmes événements mais pour des raisons sans rapport — recalculer une valeur
/// d'un côté, oublier un panier de l'autre. Les fusionner ferait un listener à
/// deux responsabilités, qu'on n'oserait plus toucher.
fn ends_in_ready_to_play(event: &TeamDomainEvent) -> bool {
    matches!(
        event,
        TeamDomainEvent::TeamEnrolled { .. }
            | TeamDomainEvent::DismissalsPhaseValidated
            | TeamDomainEvent::MatchReportingCancelled { .. }
            | TeamDomainEvent::CostlyMistakesApplied { .. }
    )
}

/// Purge les deux brouillons dès que l'équipe repasse « prête à jouer ».
///
/// Un brouillon ne survit donc jamais à un tour de séquence : le coach qui
/// revient trouve une page vierge, jamais des lignes fantômes d'un après-match
/// précédent.
///
/// Listener **intra-BC** : la signature `init(event_bus: ...)` est la convention
/// que `check-arch` (axe 5) utilise pour le distinguer d'un listener cross-BC.
pub fn init(event_bus: &EventBus, drafts: Arc<dyn IPhaseDraftRepository>) {
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
                    if !ends_in_ready_to_play(&event) {
                        continue;
                    }
                    let team_id = envelope.emitter.clone();
                    for phase in [GamePhase::Recruitment, GamePhase::Dismissals] {
                        if let Err(e) = drafts.delete(&team_id, &phase).await {
                            tracing::error!(
                                "phase_draft_purge_listener: purge {phase:?} de {team_id} : {e}"
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("phase_draft_purge_listener: lagged by {n}");
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
    use crate::app::teams::ports::PhaseDraftState;

    #[test]
    fn les_quatre_entrees_en_ready_to_play_purgent() {
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

    /// Valider le recrutement fait passer en phase de renvois, pas en
    /// « prête à jouer » : le brouillon de renvois qui vient d'être ouvert ne
    /// doit surtout pas être purgé.
    #[test]
    fn valider_le_recrutement_ne_purge_pas() {
        assert!(!ends_in_ready_to_play(
            &TeamDomainEvent::RecruitmentPhaseValidated
        ));
    }

    // ── Purge de bout en bout ────────────────────────────────────────────

    async fn test_pool() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
    }

    fn brouillon(team_id: &str, phase: GamePhase) -> PhaseDraftState {
        PhaseDraftState {
            team_id: team_id.to_string(),
            space_id: "space-1".to_string(),
            phase,
            state: serde_json::json!(["une ligne"]),
            version: 0,
        }
    }

    /// L'effet utile de la carte : un brouillon ne survit pas à un tour de
    /// séquence. Le coach qui revient trouve une page vierge, jamais des lignes
    /// fantômes de l'après-match précédent.
    #[tokio::test]
    async fn une_entree_en_ready_to_play_purge_les_deux_brouillons() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo: Arc<dyn IPhaseDraftRepository> = Arc::new(
            crate::app::teams::io::repository::phase_draft_repository::PhaseDraftRepository::new(
                pool.clone(),
            ),
        );
        let team_id = ulid::Ulid::new().to_string();

        repo.save(&brouillon(&team_id, GamePhase::Recruitment), 0)
            .await
            .unwrap();
        repo.save(&brouillon(&team_id, GamePhase::Dismissals), 0)
            .await
            .unwrap();

        let bus = crate::common::services::event_bus::event_bus::new_bus();
        init(&bus, repo.clone());

        let _ = bus.send(TeamDomainEvent::DismissalsPhaseValidated.to_enveloppe(&team_id));

        // Le listener travaille dans sa propre tâche : on attend qu'il ait fini
        // plutôt que de supposer un ordonnancement.
        for _ in 0..40 {
            let reste = repo
                .load(&team_id, &GamePhase::Recruitment)
                .await
                .unwrap()
                .is_some();
            if !reste {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }

        assert!(repo
            .load(&team_id, &GamePhase::Recruitment)
            .await
            .unwrap()
            .is_none());
        assert!(
            repo.load(&team_id, &GamePhase::Dismissals)
                .await
                .unwrap()
                .is_none(),
            "les deux brouillons partent, pas seulement celui de la phase quittée"
        );
    }
}
