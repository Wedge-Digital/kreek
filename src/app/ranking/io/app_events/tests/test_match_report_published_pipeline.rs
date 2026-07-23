//! Test d'intégration bout-en-bout : publication d'un `MatchReportPublished`
//! sur un vrai `EventBus`, écouté par le vrai listener (`match_report_published_listener`),
//! tournant en tâche de fond contre une vraie `PgPool` — pas de mock, pas de navigateur.

#![cfg(test)]

use crate::app::ranking::context::RankingContext;
use crate::app::ranking::io::app_events::match_report_published_listener;
use crate::app::ranking::ports::{
    EnrolledTeamInfo, IRankingCompetitionPort, RankingRulesInfo,
};
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportPublishedPayload;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::common_types::SeasonId;
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::new_bus;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

struct FakeCompetitionPort;

#[async_trait::async_trait]
impl IRankingCompetitionPort for FakeCompetitionPort {
    async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
        Some(RankingRulesInfo { win_points: 3, draw_points: 1, lose_points: 0 })
    }
    async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
        vec![]
    }
    async fn find_groups(&self, _: &str) -> Vec<crate::app::ranking::ports::RankingGroupInfo> {
        vec![]
    }
}

fn sample_payload(season_id: &str, home_team_id: &str, away_team_id: &str) -> MatchReportPublishedPayload {
    MatchReportPublishedPayload {
        match_report_id: crate::app::shared_kernel::common_types::MatchReportId::new().to_string(),
        space_id: "sp1".into(),
        competition_id: crate::app::shared_kernel::common_types::CompetitionId::new().to_string(),
        season_id: season_id.to_string(),
        round_id: crate::app::shared_kernel::common_types::RoundId::new().to_string(),
        pairing_id: None,
        published_at: chrono::Utc::now(),
        home_team_id: home_team_id.to_string(),
        away_team_id: away_team_id.to_string(),
        home_score: 2,
        away_score: 1,
        home_gain_kpo: 0,
        away_gain_kpo: 0,
        home_fan_mod: 0,
        away_fan_mod: 0,
        home_actions: vec![],
        away_actions: vec![],
        home_temp_players: vec![],
        away_temp_players: vec![],
    }
}

async fn wait_for<F, Fut>(mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..50 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition never satisfied within timeout");
}

#[sqlx::test]
async fn match_report_published_creates_two_ranking_lines(pool: PgPool) {
    let app_event_bus = new_bus();
    let competition_port: Arc<dyn IRankingCompetitionPort> = Arc::new(FakeCompetitionPort);
    let ranking = RankingContext::new(&pool, competition_port.clone());
    match_report_published_listener::init(&app_event_bus, ranking.repository.clone(), competition_port);

    let season_id = SeasonId::new();
    let home = TeamId::new();
    let away = TeamId::new();
    let payload = sample_payload(&season_id.to_string(), &home.to_string(), &away.to_string());

    let _ = app_event_bus.send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());

    wait_for(|| {
        let repo = ranking.repository.clone();
        let season = season_id.to_string();
        let home = home.to_string();
        async move { repo.find_latest_line(&season, &home).await.unwrap().is_some() }
    })
    .await;

    let home_row = ranking.repository.find_latest_line(&season_id.to_string(), &home.to_string()).await.unwrap().unwrap();
    let away_row = ranking.repository.find_latest_line(&season_id.to_string(), &away.to_string()).await.unwrap().unwrap();

    assert_eq!(home_row.ranking_points, 3); // victoire 2-1
    assert_eq!(away_row.ranking_points, 0); // défaite
    assert_eq!(home_row.matches_played, 1);
}
