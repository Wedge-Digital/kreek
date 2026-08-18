use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
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
                    let MatchReportAppEvent::MatchReportConfirmed {
                        match_report_id,
                        home_team_id,
                        away_team_id,
                        ..
                    } = app_event
                    else {
                        continue;
                    };

                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    // Le contrôle de l'identifiant est dans le span, sans quoi
                    // son avertissement — le seul signe qu'un événement a été
                    // écarté — ne se rattacherait à rien. Le `for`, lui, est
                    // enveloppé en entier : ses `continue` visent la boucle
                    // interne, jamais celle de la souscription.
                    async {
                    let mr_id = match MatchReportId::try_new(&match_report_id) {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::warn!(
                                "match_report_confirmed_listener: invalid match_report_id {match_report_id}: {e}"
                            );
                            return;
                        }
                    };

                    for team_id in [&home_team_id, &away_team_id] {
                        let team = match team_repo.find_by_id(team_id).await {
                            Ok(Some(t)) => t,
                            Ok(None) => {
                                tracing::warn!(
                                    "match_report_confirmed_listener: team {team_id} not found"
                                );
                                continue;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "match_report_confirmed_listener: find_by_id {team_id}: {e}"
                                );
                                continue;
                            }
                        };

                        let event = match team.start_match_reporting(mr_id) {
                            Ok(e) => e,
                            Err(e) => {
                                tracing::warn!(
                                    "match_report_confirmed_listener: start_match_reporting {team_id}: {e}"
                                );
                                continue;
                            }
                        };

                        if let Err(e) = team_repo.append(team_id, &event, team.version).await {
                            tracing::error!(
                                "match_report_confirmed_listener: append {team_id}: {e}"
                            );
                        } else {
                            tracing::info!(
                                "match_report_confirmed_listener: team {team_id} → MatchReporting"
                            );
                        }
                    }
                    }
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_confirmed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
