//! Le recalcul de la valeur d'équipe, et **tout** ce qui le déclenche.
//!
//! La règle tenait dans deux fichiers — quatre événements domaine ici, un app
//! event dans `io/app_events/initial_roster_listener.rs` — et la carte 270 en
//! aurait ajouté un troisième. Trois endroits pour une règle dont aucun ne
//! portait la liste entière.
//!
//! Elle est donc rassemblée ici, ce qui laisse **deux** fonctions d'abonnement
//! plutôt qu'une : la convention `init(event_bus: …)` / `init(app_event_bus: …)`
//! est ce dont l'axe 5 de `check-arch` se sert pour distinguer un listener
//! intra-BC d'un listener cross-BC, et les fondre en une seule signature
//! brouillerait ce verrou pour un gain d'écriture. Le fichier est rangé par ce
//! qu'il entretient — la valeur d'équipe — et non par le bus qui l'alimente.

use crate::app::shared_kernel::app_events::players_app_events::PlayersAppEvent;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::ports::{
    IJourneymanTypePort, IRosterCatalogPort, ISquadPort, ITeamRepository,
};
use crate::app::teams::use_cases::recompute_team_value_use_case;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

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

/// Les faits que `players` annonce et qui changent l'effectif **une fois écrits**.
///
/// Ce sont les seuls moments où un recalcul lit un effectif à jour. Les deux
/// existent pour la même raison : `teams` et `players` réagissent au même
/// événement dans deux tâches indépendantes, et la lecture de `teams` arrive
/// systématiquement avant l'écriture de `players`. Sans ces annonces, une équipe
/// se figerait à une TV de zéro à la création, et compterait ses renvoyés après
/// la validation des renvois.
fn changes_squad(event: &PlayersAppEvent) -> Option<&str> {
    match event {
        PlayersAppEvent::InitialRosterCompleted { team_id, .. }
        | PlayersAppEvent::PlayerDismissed { team_id, .. }
        // Un commissaire a posé la valeur d'un joueur hors barème. L'effectif
        // ne change pas, sa valeur si — et c'est bien la TV qu'il faut relire.
        // `players` n'annonce que le **prix** : compétence et caractéristique
        // customisées ne déplacent pas la valeur d'équipe.
        | PlayersAppEvent::PlayerValueCustomised { team_id, .. } => Some(team_id),
    }
}

/// Les quatre ports dont le recalcul a besoin, portés ensemble pour que les deux
/// abonnements ne dupliquent pas la même liste de paramètres.
#[derive(Clone)]
pub struct TeamValueDeps {
    pub repo: Arc<dyn ITeamRepository>,
    pub squad_port: Arc<dyn ISquadPort>,
    pub roster_catalog_port: Arc<dyn IRosterCatalogPort>,
    pub journeyman_type_port: Arc<dyn IJourneymanTypePort>,
}

async fn recalculer(team_id: &str, deps: &TeamValueDeps, source: &str) {
    if let Err(e) = recompute_team_value_use_case::execute(
        team_id,
        deps.repo.as_ref(),
        deps.squad_port.as_ref(),
        deps.roster_catalog_port.as_ref(),
        deps.journeyman_type_port.as_ref(),
    )
    .await
    {
        tracing::error!("team_value_listener [{source}] : recalcul de {team_id} : {e:?}");
    }
}

/// Listener **intra-BC** : il écoute le bus interne de `teams`, alimenté par
/// `TeamRepository::append`. La signature `init(event_bus: ...)` est la
/// convention que `check-arch` (axe 5) utilise pour le distinguer d'un listener
/// cross-BC — ne pas la renommer sans lire cet axe.
pub fn init(event_bus: &EventBus, deps: TeamValueDeps) {
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
                    // `TeamValueRecomputed` n'est surtout pas un déclencheur :
                    // le recalcul appende, l'append publie, et le listener
                    // recevrait son propre événement — boucle infinie.
                    if !ends_in_ready_to_play(&event) {
                        continue;
                    }
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    recalculer(&envelope.emitter.clone(), &deps, "domaine")
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("team_value_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Listener **cross-BC** : il écoute ce que `players` annonce une fois son
/// effectif écrit. La signature `init_from_app_events(app_event_bus: ...)` porte
/// la même convention que `init`, et l'exempte de la règle de transaction unique
/// — l'événement vient d'un commit déjà passé dans un autre BC.
///
/// Aucun ordre n'est à garantir vis-à-vis de l'autre abonnement :
/// `TeamValueRecomputed` porte une valeur **absolue**, donc un recalcul
/// prématuré est écrasé par celui-ci. C'est ce qui rend inoffensif le recalcul
/// que `DismissalsPhaseValidated` déclenche trop tôt.
pub fn init_from_app_events(app_event_bus: &EventBus, deps: TeamValueDeps) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<PlayersAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let Some(team_id) = changes_squad(&event) else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    recalculer(&team_id.to_string(), &deps, "players")
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("team_value_listener [players]: lagged by {n}");
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
                damage_dice: vec![],
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

    /// Les annonces de `players` recalculent, et nomment l'équipe — pas le
    /// joueur : c'est l'équipe dont la valeur bouge.
    #[test]
    fn les_annonces_de_players_declenchent_le_recalcul() {
        assert_eq!(
            changes_squad(&PlayersAppEvent::InitialRosterCompleted {
                team_id: "t-1".into(),
                player_count: 11,
            }),
            Some("t-1")
        );
        assert_eq!(
            changes_squad(&PlayersAppEvent::PlayerDismissed {
                team_id: "t-2".into(),
                player_id: "p-9".into(),
            }),
            Some("t-2")
        );
    }

    /// Un prix posé par un commissaire ne change pas l'effectif, mais change sa
    /// valeur — et c'est bien la TV qu'il faut relire.
    ///
    /// `players` ne fait sortir **que** le prix : compétence et caractéristique
    /// customisées n'ont pas d'app event, donc rien à filtrer ici. C'est le
    /// test du publisher, côté `players`, qui tient cette moitié de la règle.
    #[test]
    fn une_customisation_de_prix_declenche_le_recalcul() {
        assert_eq!(
            changes_squad(&PlayersAppEvent::PlayerValueCustomised {
                team_id: "t-3".into(),
                player_id: "p-7".into(),
            }),
            Some("t-3")
        );
    }
}
