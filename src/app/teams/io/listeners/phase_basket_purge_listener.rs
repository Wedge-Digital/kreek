use crate::app::teams::domain::team::{GamePhase, TeamDomainEvent};
use crate::app::teams::ports::IPhaseBasketRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

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

/// Purge les deux paniers dès que l'équipe repasse « prête à jouer ».
///
/// Un panier ne survit donc jamais à un tour de séquence : le coach qui
/// revient trouve une page vierge, jamais des lignes fantômes d'un après-match
/// précédent.
///
/// Listener **intra-BC** : la signature `init(event_bus: ...)` est la convention
/// que `check-arch` (axe 5) utilise pour le distinguer d'un listener cross-BC.
pub fn init(event_bus: &EventBus, baskets: Arc<dyn IPhaseBasketRepository>) {
    let mut rx = event_bus.subscribe();
    spawn_listener(module_path!(), async move {
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
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    let team_id = envelope.emitter.clone();
                    async {
                        for phase in [GamePhase::Recruitment, GamePhase::Dismissals] {
                            if let Err(e) = baskets.delete(&team_id, &phase).await {
                                tracing::error!(
                                    "phase_basket_purge_listener: purge {phase:?} de {team_id} : {e}"
                                );
                            }
                        }
                    }
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("phase_basket_purge_listener: lagged by {n}");
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
    use crate::app::teams::ports::PhaseBasketState;

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
    /// « prête à jouer » : le panier de renvois qui vient d'être ouvert ne
    /// doit surtout pas être purgé.
    #[test]
    fn valider_le_recrutement_ne_purge_pas() {
        assert!(!ends_in_ready_to_play(
            &TeamDomainEvent::RecruitmentPhaseValidated
        ));
    }

    // ── Purge de bout en bout ────────────────────────────────────────────

    fn panier(team_id: &str, phase: GamePhase) -> PhaseBasketState {
        PhaseBasketState {
            team_id: team_id.to_string(),
            space_id: "space-1".to_string(),
            phase,
            state: serde_json::json!(["une ligne"]),
            version: 0,
        }
    }

    /// L'effet utile de la carte : un panier ne survit pas à un tour de
    /// séquence. Le coach qui revient trouve une page vierge, jamais des lignes
    /// fantômes de l'après-match précédent.
    /// `#[sqlx::test]` plutôt qu'un pool monté à la main sur `DATABASE_URL`.
    /// L'ancienne forme rendait `None` quand la variable manquait, et le test
    /// **passait alors sans rien vérifier** — un vert qui ne prouve rien est
    /// pire qu'un rouge. Elle partageait par ailleurs la base des autres tests
    /// avec deux connexions pour le test *et* le listener.
    #[sqlx::test]
    async fn une_entree_en_ready_to_play_purge_les_deux_paniers(pool: sqlx::PgPool) {
        let repo: Arc<dyn IPhaseBasketRepository> = Arc::new(
            crate::app::teams::io::repository::phase_basket_repository::PhaseBasketRepository::new(
                pool.clone(),
            ),
        );
        let team_id = ulid::Ulid::new().to_string();

        repo.save(&panier(&team_id, GamePhase::Recruitment), 0)
            .await
            .unwrap();
        repo.save(&panier(&team_id, GamePhase::Dismissals), 0)
            .await
            .unwrap();

        let bus = crate::common::services::event_bus::event_bus::new_bus();
        init(&bus, repo.clone());

        let _ = bus.send(TeamDomainEvent::DismissalsPhaseValidated.to_enveloppe(&team_id));

        // Le listener travaille dans sa propre tâche : on attend qu'il ait fini
        // plutôt que de supposer un ordonnancement.
        //
        // Et on attend **les deux** paniers, jamais le premier seul. Le
        // listener les supprime en deux `await` successifs ; observer le
        // premier faisait sortir de cette boucle **entre les deux
        // suppressions**, et l'assertion sur le second tombait sur un panier
        // qui allait disparaître un instant plus tard. Environ un échec sur dix
        // en suite complète, aucun en isolation — la charge n'était pas la
        // cause mais l'amplificateur.
        for _ in 0..80 {
            let reste = repo
                .load(&team_id, &GamePhase::Recruitment)
                .await
                .unwrap()
                .is_some()
                || repo
                    .load(&team_id, &GamePhase::Dismissals)
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
            "les deux paniers partent, pas seulement celui de la phase quittée"
        );
    }
}
