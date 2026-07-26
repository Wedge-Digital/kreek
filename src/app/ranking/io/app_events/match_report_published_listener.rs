use crate::app::ranking::domain::ranking_line::{CasualtiesInflicted, MatchScore};
use crate::app::ranking::ports::IRankingCompetitionPort;
use crate::app::ranking::use_cases::record_match_ranking_use_case::{
    self, RecordMatchRankingCommand, TeamMatchStats,
};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
};
use crate::app::shared_kernel::common_types::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(
    app_event_bus: &EventBus,
    repo: Arc<dyn crate::app::ranking::ports::IRankingRepository>,
    competition_port: Arc<dyn IRankingCompetitionPort>,
) {
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
                    let MatchReportAppEvent::MatchReportPublished(payload) = app_event else {
                        continue;
                    };
                    handle_published(payload, repo.as_ref(), competition_port.as_ref()).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("ranking::match_report_published_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_published(
    payload: crate::app::shared_kernel::app_events::match_report_app_events::MatchReportPublishedPayload,
    repo: &dyn crate::app::ranking::ports::IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) {
    let Ok(competition_id) = CompetitionId::try_new(&payload.competition_id) else { return };
    let Ok(season_id) = SeasonId::try_new(&payload.season_id) else { return };
    let Ok(round_id) = RoundId::try_new(&payload.round_id) else { return };
    let Ok(match_report_id) = MatchReportId::try_new(&payload.match_report_id) else { return };
    let Ok(home_team_id) = TeamId::try_new(&payload.home_team_id) else { return };
    let Ok(away_team_id) = TeamId::try_new(&payload.away_team_id) else { return };

    let cmd = RecordMatchRankingCommand {
        competition_id,
        season_id,
        round_id,
        match_report_id: match_report_id.clone(),
        home_team_id,
        away_team_id,
        home: TeamMatchStats {
            score: MatchScore(payload.home_score),
            casualties: count_sorties(&payload.home_actions),
        },
        away: TeamMatchStats {
            score: MatchScore(payload.away_score),
            casualties: count_sorties(&payload.away_actions),
        },
        published_at: payload.published_at,
    };

    if let Err(e) = record_match_ranking_use_case::execute(cmd, repo, competition_port).await {
        tracing::error!(
            "ranking::match_report_published_listener: échec pour {}: {e:?}",
            match_report_id
        );
    }
}

/// Compte les sorties (`Sortie` seule) infligées par une équipe — filtrage IO :
/// le domaine ne connaît pas les types du payload, il reçoit un nombre.
fn count_sorties(actions: &[MatchActionPublishedPayload]) -> CasualtiesInflicted {
    let count = actions
        .iter()
        .filter(|a| matches!(a.action, ActionTypePayload::Sortie))
        .count();
    CasualtiesInflicted(count as u32)
}
