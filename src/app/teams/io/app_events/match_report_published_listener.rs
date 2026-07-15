use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, MatchReportPublishedPayload,
};
use crate::app::teams::domain::value_objects::{Kpo, MatchResult};
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::cmp::Ordering;
use std::sync::Arc;

/// Conséquences du rapport publié pour une équipe — jamais construit en
/// mélangeant les champs home/away (cf. `derive_team_effects`).
struct TeamMatchEffect {
    team_id: String,
    result: MatchResult,
    fan_mod: i8,
    gain_kpo: u32,
}

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

                    let (home, away) = derive_team_effects(&payload);
                    handle_team(&team_repo, home).await;
                    handle_team(&team_repo, away).await;
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

fn derive_team_effects(
    payload: &MatchReportPublishedPayload,
) -> (TeamMatchEffect, TeamMatchEffect) {
    let home = TeamMatchEffect {
        team_id: payload.home_team_id.clone(),
        result: derive_result(payload.home_score, payload.away_score),
        fan_mod: payload.home_fan_mod,
        gain_kpo: payload.home_gain_kpo,
    };
    let away = TeamMatchEffect {
        team_id: payload.away_team_id.clone(),
        result: derive_result(payload.away_score, payload.home_score),
        fan_mod: payload.away_fan_mod,
        gain_kpo: payload.away_gain_kpo,
    };
    (home, away)
}

async fn handle_team(team_repo: &Arc<dyn ITeamRepository>, effect: TeamMatchEffect) {
    let team_id = &effect.team_id;
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

    // spp_gains reste hors périmètre (cartes 35/145/154).
    match team.start_post_match_sequence(effect.result, effect.fan_mod, Kpo(effect.gain_kpo), vec![]) {
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

    fn sample_payload() -> MatchReportPublishedPayload {
        MatchReportPublishedPayload {
            match_report_id: "mr1".into(), space_id: "sp1".into(), competition_id: "c1".into(),
            season_id: "s1".into(), round_id: "r1".into(), pairing_id: None,
            published_at: chrono::Utc::now(),
            home_team_id: "home-team".into(), away_team_id: "away-team".into(),
            home_score: 2, away_score: 1,
            home_gain_kpo: 150_000, away_gain_kpo: 90_000,
            home_fan_mod: 1, away_fan_mod: -2,
            home_actions: vec![], away_actions: vec![], home_temp_players: vec![], away_temp_players: vec![],
        }
    }

    #[test]
    fn derive_team_effects_never_crosses_home_and_away() {
        let (home, away) = derive_team_effects(&sample_payload());

        assert_eq!(home.team_id, "home-team");
        assert_eq!(home.result, MatchResult::Win);
        assert_eq!(home.fan_mod, 1);
        assert_eq!(home.gain_kpo, 150_000);

        assert_eq!(away.team_id, "away-team");
        assert_eq!(away.result, MatchResult::Loss);
        assert_eq!(away.fan_mod, -2);
        assert_eq!(away.gain_kpo, 90_000);
    }
}
