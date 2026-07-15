use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::teams::domain::value_objects::{Kpo, MatchResult};
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::cmp::Ordering;
use std::sync::Arc;

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportPublished(payload)) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };

                    handle_team(
                        &team_repo,
                        &payload.home_team_id,
                        derive_result(payload.home_score, payload.away_score),
                    )
                    .await;
                    handle_team(
                        &team_repo,
                        &payload.away_team_id,
                        derive_result(payload.away_score, payload.home_score),
                    )
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_published_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn derive_result(own_score: u8, opponent_score: u8) -> MatchResult {
    match own_score.cmp(&opponent_score) {
        Ordering::Greater => MatchResult::Win,
        Ordering::Equal => MatchResult::Draw,
        Ordering::Less => MatchResult::Loss,
    }
}

async fn handle_team(team_repo: &Arc<dyn ITeamRepository>, team_id: &str, result: MatchResult) {
    let team = match team_repo.find_by_id(team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!("match_report_published_listener: team {team_id} not found");
            return;
        }
        Err(e) => {
            tracing::error!("match_report_published_listener: find_by_id {team_id}: {e}");
            return;
        }
    };

    // Câblage minimal : les valeurs réelles (revenus, jet de fans, SPP) sont
    // stubbées, calcul complet hors périmètre (cartes 35/145/154).
    match team.start_post_match_sequence(result, 0, Kpo(0), vec![]) {
        Ok(event) => {
            if let Err(e) = team_repo.append(team_id, &event, team.version).await {
                tracing::error!("match_report_published_listener: append {team_id}: {e}");
            } else {
                tracing::info!(
                    "match_report_published_listener: team {team_id} → PlayerImprovement"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                "match_report_published_listener: start_post_match_sequence {team_id}: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_result_victoire() {
        assert_eq!(derive_result(2, 1), MatchResult::Win);
    }

    #[test]
    fn derive_result_defaite() {
        assert_eq!(derive_result(1, 2), MatchResult::Loss);
    }

    #[test]
    fn derive_result_nul() {
        assert_eq!(derive_result(1, 1), MatchResult::Draw);
    }
}
