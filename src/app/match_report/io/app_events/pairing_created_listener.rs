use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::match_report::use_cases::create_match_report_use_case;
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::common_types::{CompetitionId, RoundId, SeasonId, SpaceId};
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(app_event_bus: &EventBus, repo: Arc<dyn IMatchReportRepository>) {
    let bus = app_event_bus.clone();
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<CompetitionsAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let CompetitionsAppEvent::PairingCreated {
                        pairing_id,
                        competition_id,
                        season_id,
                        round_id,
                        home_team_id,
                        away_team_id,
                        space_id,
                        ..
                    } = app_event
                    else {
                        continue;
                    };

                    let Ok(space) = SpaceId::try_new(&space_id) else { continue };
                    let Ok(comp) = CompetitionId::try_new(&competition_id) else { continue };
                    let Ok(season) = SeasonId::try_new(&season_id) else { continue };
                    let Ok(round) = RoundId::try_new(&round_id) else { continue };
                    let Ok(home) = TeamId::try_new(&home_team_id) else { continue };
                    let Ok(away) = TeamId::try_new(&away_team_id) else { continue };
                    let Ok(coach) = CompetitionId::try_new(&competition_id) else { continue };

                    let cmd = create_match_report_use_case::CreateMatchReportCommand {
                        space_id: space,
                        competition_id: comp,
                        season_id: season,
                        round_id: round,
                        home_team_id: home,
                        away_team_id: away,
                        created_by: coach,
                        origin: MatchReportOrigin::Pairing,
                        pairing_id: Some(pairing_id),
                    };

                    match create_match_report_use_case::execute(cmd, repo.as_ref(), &bus).await {
                        Ok(mr_id) => {
                            tracing::info!(
                                "pairing_created_listener: created match report {mr_id} for pairing"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "pairing_created_listener: failed to create match report: {e:?}"
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("pairing_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
