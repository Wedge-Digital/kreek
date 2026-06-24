use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
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
                    } = app_event;

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

                        let event = match team.start_match_reporting(match_report_id.clone()) {
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
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_confirmed_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
