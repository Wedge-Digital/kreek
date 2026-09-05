//! Un journalier aligné dans un rapport devient un joueur de l'effectif.
//!
//! # Pourquoi ce détour par `teams`
//!
//! `match_report` constate un fait de match — on a aligné des journaliers. Il
//! ne crée pas de joueurs, ce n'est pas son rôle. `teams` en tire la
//! conséquence d'effectif, et **reste le seul BC à faire naître un joueur** :
//! le chemin `teams → players` demeure unique, et l'event store de `teams`
//! raconte l'histoire complète de son effectif.
//!
//! # Aligner n'est pas recruter
//!
//! `JourneymanFielded` et non `PlayerRecruited`, pour deux raisons
//! indépendantes qui disent la même chose. `recruit_player` ouvre par
//! `expect_phase(Recruitment)` — un journalier est aligné en `MatchReporting`,
//! l'appel serait rejeté systématiquement. Et `PlayerRecruited` produit un
//! débit de trésorerie : à coût nul rien n'est prélevé, mais rien ne filtre
//! les mouvements nuls, et le grand livre aurait gagné une ligne
//! « Recrutement de joueur — 0 kPo » par journalier et par match.
//!
//! # Le retrait n'est pas un luxe
//!
//! `TempPlayersInitialized` est émis à chaque enregistrement des coups de
//! pouce, et le coach peut y revenir : les journaliers sont alors refrappés
//! avec de nouveaux identifiants. Sans le retrait symétrique, les premiers
//! resteraient dans l'effectif, occupant leur maillot pour un match où plus
//! personne ne les aligne.
//!
//! `init(app_event_bus: …)` : c'est cette signature que l'axe 5 de
//! `check-arch` reconnaît comme listener cross-BC.

use crate::app::shared_kernel::app_events::match_report_app_events::{
    FieldedJourneyman, MatchReportAppEvent,
};
use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::basket::RosterLineId;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    match app_event {
                        MatchReportAppEvent::JourneymenFielded {
                            team_id, players, ..
                        } => {
                            aligner(&team_id, players, team_repo.as_ref())
                                .instrument(span)
                                .await
                        }
                        MatchReportAppEvent::JourneymenWithdrawn {
                            team_id,
                            player_ids,
                            ..
                        } => {
                            retirer(&team_id, player_ids, team_repo.as_ref())
                                .instrument(span)
                                .await
                        }
                        _ => continue,
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams::journeymen_fielded_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Chaque journalier est appendu séparément, en rechargeant l'équipe.
///
/// La version optimiste de l'agrégat interdit d'appender deux événements sur
/// la même : les appender en lot sans relire ferait perdre tous les suivants
/// en silence.
async fn aligner(team_id: &str, players: Vec<FieldedJourneyman>, repo: &dyn ITeamRepository) {
    for player in players {
        let Ok(player_id) = PlayerId::try_new(&player.player_id) else {
            tracing::warn!(
                "teams::journeymen_fielded_listener: identifiant illisible {}",
                player.player_id
            );
            continue;
        };
        let event = TeamDomainEvent::JourneymanFielded {
            player_id,
            roster_line: RosterLineId(player.roster_line_id.clone()),
        };
        appendre(team_id, &event, repo).await;
    }
}

async fn retirer(team_id: &str, player_ids: Vec<String>, repo: &dyn ITeamRepository) {
    for id in player_ids {
        let Ok(player_id) = PlayerId::try_new(&id) else {
            tracing::warn!("teams::journeymen_fielded_listener: identifiant illisible {id}");
            continue;
        };
        appendre(
            team_id,
            &TeamDomainEvent::JourneymanWithdrawn { player_id },
            repo,
        )
        .await;
    }
}

/// Recharge, appende, et journalise l'échec sans interrompre les suivants :
/// l'échec d'un journalier ne doit pas priver les autres de leur naissance.
async fn appendre(team_id: &str, event: &TeamDomainEvent, repo: &dyn ITeamRepository) {
    let team = match repo.find_by_id(team_id).await {
        Ok(Some(team)) => team,
        Ok(None) => {
            tracing::warn!("teams::journeymen_fielded_listener: équipe {team_id} introuvable");
            return;
        }
        Err(e) => {
            tracing::error!("teams::journeymen_fielded_listener: chargement {team_id}: {e}");
            return;
        }
    };
    let version = team.version;
    if let Err(e) = repo.append(team_id, event, version).await {
        tracing::error!("teams::journeymen_fielded_listener: append {team_id}: {e}");
    }
}
