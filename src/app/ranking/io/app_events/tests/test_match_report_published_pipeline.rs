//! Test d'intégration bout-en-bout : publication d'un `MatchReportPublished`
//! sur un vrai `EventBus`, écouté par le vrai listener (`match_report_published_listener`),
//! tournant en tâche de fond contre une vraie `PgPool` — pas de mock, pas de navigateur.

#![cfg(test)]

use crate::app::ranking::context::RankingContext;
use crate::app::ranking::io::app_events::match_report_published_listener;
use crate::app::ranking::ports::{
    BonusRuleInfo, EnrolledTeamInfo, IRankingCompetitionPort, RankingRulesInfo,
};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload, PlayerRefPayload,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::common::services::event_bus::event_bus::new_bus;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

fn disabled_bonus() -> BonusRuleInfo {
    BonusRuleInfo {
        activated: false,
        threshold: 0,
        points: 0,
    }
}

struct FakeCompetitionPort;

#[async_trait::async_trait]
impl IRankingCompetitionPort for FakeCompetitionPort {
    async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
        Some(RankingRulesInfo {
            win_points: 3,
            draw_points: 1,
            lose_points: 0,
            offensive: disabled_bonus(),
            defensive: disabled_bonus(),
            // Bonus agressif activé : +1 point si > 1 sortie infligée.
            aggressive: BonusRuleInfo {
                activated: true,
                threshold: 1,
                points: 1,
            },
            tiebreakers: vec![],
        })
    }
    async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
        vec![]
    }
    async fn find_groups(&self, _: &str) -> Vec<crate::app::ranking::ports::RankingGroupInfo> {
        vec![]
    }
}

fn sample_payload(
    season_id: &str,
    home_team_id: &str,
    away_team_id: &str,
) -> MatchReportPublishedPayload {
    MatchReportPublishedPayload {
        match_report_id: crate::app::shared_kernel::bloodbowl::ids::MatchReportId::new()
            .to_string(),
        space_id: "sp1".into(),
        competition_id: crate::app::shared_kernel::bloodbowl::ids::CompetitionId::new().to_string(),
        season_id: season_id.to_string(),
        round_id: crate::app::shared_kernel::bloodbowl::ids::RoundId::new().to_string(),
        pairing_id: None,
        published_at: chrono::Utc::now(),
        home_team_id: home_team_id.to_string(),
        away_team_id: away_team_id.to_string(),
        home_score: 2,
        away_score: 1,
        home_gain_kpo: 0,
        home_inducement_spending_kpo: 0,
        away_inducement_spending_kpo: 0,
        away_gain_kpo: 0,
        home_fan_mod: 0,
        away_fan_mod: 0,
        home_actions: vec![sortie(), sortie()], // 2 sorties > seuil (1) → bonus agressif
        away_actions: vec![],
        home_temp_players: vec![],
        away_temp_players: vec![],
    }
}

fn action(kind: ActionTypePayload) -> MatchActionPublishedPayload {
    MatchActionPublishedPayload {
        turn: 1,
        player: PlayerRefPayload::Regular {
            player_id: "p1".into(),
        },
        action: kind,
    }
}

fn sortie() -> MatchActionPublishedPayload {
    action(ActionTypePayload::Sortie)
}

/// `n` actions du même type — pour composer un payload aux compteurs distincts.
fn actions(kind: ActionTypePayload, n: usize) -> Vec<MatchActionPublishedPayload> {
    (0..n).map(|_| action(kind.clone())).collect()
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

/// Ce banc n'exerce aucune autorisation : il vérifie le tuyau app event →
/// lignes de classement. La doublure refuse tout, ce qui est le défaut sûr.
struct FakeAdmin;

#[async_trait::async_trait]
impl crate::app::ranking::ports::IRankingAdminPort for FakeAdmin {
    async fn is_competition_admin(&self, _: &str, _: &str) -> bool {
        false
    }
    async fn is_space_admin(&self, _: &str, _: &str) -> bool {
        false
    }
}

#[sqlx::test]
async fn match_report_published_creates_two_ranking_lines(pool: PgPool) {
    let app_event_bus = new_bus();
    let competition_port: Arc<dyn IRankingCompetitionPort> = Arc::new(FakeCompetitionPort);
    let ranking = RankingContext::new(&pool, competition_port.clone(), Arc::new(FakeAdmin));
    match_report_published_listener::init(
        &app_event_bus,
        ranking.repository.clone(),
        competition_port,
    );

    let season_id = SeasonId::new();
    let home = TeamId::new();
    let away = TeamId::new();
    let payload = sample_payload(&season_id.to_string(), &home.to_string(), &away.to_string());

    let _ = app_event_bus.send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());

    wait_for(|| {
        let repo = ranking.repository.clone();
        let season = season_id.to_string();
        let home = home.to_string();
        async move {
            repo.find_latest_line(&season, &home)
                .await
                .unwrap()
                .is_some()
        }
    })
    .await;

    let home_row = ranking
        .repository
        .find_latest_line(&season_id.to_string(), &home.to_string())
        .await
        .unwrap()
        .unwrap();
    let away_row = ranking
        .repository
        .find_latest_line(&season_id.to_string(), &away.to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(home_row.ranking_points, 4); // victoire 2-1 (3) + bonus agressif (1)
    assert_eq!(away_row.ranking_points, 0); // défaite, aucune sortie
    assert_eq!(home_row.matches_played, 1);
}

/// Les cinq compteurs de départage traversent tout le pipeline : payload d'app
/// event → filtrage IO des actions → domaine → base. Les actions non comptées
/// (`Lancer`, `Interception`, `Touchdown`) sont dans le payload pour vérifier
/// qu'elles ne gonflent ni les fautes (règle 16) ni les réussites (règle 15).
#[sqlx::test]
async fn match_report_published_persists_the_tiebreak_counters(pool: PgPool) {
    let app_event_bus = new_bus();
    let competition_port: Arc<dyn IRankingCompetitionPort> = Arc::new(FakeCompetitionPort);
    let ranking = RankingContext::new(&pool, competition_port.clone(), Arc::new(FakeAdmin));
    match_report_published_listener::init(
        &app_event_bus,
        ranking.repository.clone(),
        competition_port,
    );

    let season_id = SeasonId::new();
    let home = TeamId::new();
    let away = TeamId::new();
    let mut payload = sample_payload(&season_id.to_string(), &home.to_string(), &away.to_string());
    // home : 2 sorties, 3 agressions, 1 passe (+ 1 lancer et 1 interception ignorés).
    payload.home_actions = [
        actions(ActionTypePayload::Sortie, 2),
        actions(ActionTypePayload::Agression, 3),
        actions(ActionTypePayload::Passe, 1),
        actions(ActionTypePayload::Lancer, 1),
        actions(ActionTypePayload::Interception, 1),
    ]
    .concat();
    // away : 1 agression, 4 passes, aucune sortie.
    payload.away_actions = [
        actions(ActionTypePayload::Agression, 1),
        actions(ActionTypePayload::Passe, 4),
    ]
    .concat();

    let _ = app_event_bus.send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());

    wait_for(|| {
        let repo = ranking.repository.clone();
        let season = season_id.to_string();
        let away = away.to_string();
        async move {
            repo.find_latest_line(&season, &away)
                .await
                .unwrap()
                .is_some()
        }
    })
    .await;

    let home_row = ranking
        .repository
        .find_latest_line(&season_id.to_string(), &home.to_string())
        .await
        .unwrap()
        .unwrap();
    let away_row = ranking
        .repository
        .find_latest_line(&season_id.to_string(), &away.to_string())
        .await
        .unwrap()
        .unwrap();

    // Match 2-1 : les TD se croisent, les trois autres compteurs restent par équipe.
    assert_eq!(
        [
            home_row.td_for,
            home_row.td_against,
            home_row.casualties,
            home_row.fouls,
            home_row.completions
        ],
        [2, 1, 2, 3, 1]
    );
    assert_eq!(
        [
            away_row.td_for,
            away_row.td_against,
            away_row.casualties,
            away_row.fouls,
            away_row.completions
        ],
        [1, 2, 0, 1, 4]
    );
}
