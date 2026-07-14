//! Test d'intégration bout-en-bout du pipeline "impact des rapports de match" côté
//! `players` : publication d'app events réels sur un vrai `EventBus`, écoutés par
//! les vrais listeners (`player_match_impact_listener`, `team_match_concluded_listener`)
//! tournant en tâches de fond contre une vraie `PgPool` — pas de mock, pas de navigateur
//! (feature 100% backend, cf. `docs/specs/player-match-impact/player-report-events/07-integration.md` §7).
//!
//! Portée assumée : ce test exerce le pipeline `app_event_bus → players`, qui est la
//! partie neuve et à risque de cette feature. Le mapping `match_report` → app events
//! (BR1/BR2, quels `MatchActionType` produisent quel `PlayerMatchImpactAppEvent`) est
//! déjà couvert par des tests unitaires purs dans `match_report::io::app_events::app_event_publisher`
//! (carte 157) — reconstruire ici un agrégat `MatchReportPublished` complet via la
//! vraie chaîne de transitions du domaine `match_report` (Draft → PreMatch → ReadyToPublish
//! → Published) aurait ajouté une dépendance lourde et sans rapport avec ce qui est
//! réellement neuf dans cette carte.

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::PlayerParticipationStatus;
use crate::app::players::domain::player::{PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
use crate::app::players::io::app_events::{player_match_impact_listener, team_match_concluded_listener};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::ports::IPlayerRepository;
use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
use crate::app::shared_kernel::app_events::player_match_impact_app_events::{
    InjuryTypePayload, PlayerMatchContextPayload, PlayerMatchImpactAppEvent,
};
use crate::app::shared_kernel::common_types::SpaceId;
use crate::common::services::event_bus::event_bus::new_bus;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

fn context(player_id: &str) -> PlayerMatchContextPayload {
    PlayerMatchContextPayload {
        match_report_id:    "mr1".into(),
        round_id:           "r1".into(),
        round_label:        "Journée 5".into(),
        opponent_team_id:   "opponent".into(),
        opponent_team_name: "Bone Crushers".into(),
        player_id:          player_id.to_string(),
    }
}

async fn seed_player(repo: &dyn IPlayerRepository, player_id: &str, team_id: &str) {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id:      PlayerId(player_id.to_string()),
        team_id:        TeamId(team_id.to_string()),
        space_id:       SpaceId::new(),
        position_name:  PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey:         None,
        base_skills:    vec![],
        starting_spp:   Spp(0),
        starting_value: ValueKpo(100_000),
    };
    repo.append(&PlayerId(player_id.into()), &TeamId(team_id.into()), &created, 1)
        .await
        .unwrap();
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
async fn full_pipeline_credits_spp_and_records_injury_then_restores_availability(pool: PgPool) {
    let player_repo: Arc<dyn IPlayerRepository> = Arc::new(PgPlayerRepository::new(pool));
    let ref_repo = Arc::new(InMemoryReferenceRepository::load());
    let app_event_bus = new_bus();

    player_match_impact_listener::init(&app_event_bus, player_repo.clone(), ref_repo);
    team_match_concluded_listener::init(&app_event_bus, player_repo.clone());

    seed_player(player_repo.as_ref(), "scorer", "t1").await;
    seed_player(player_repo.as_ref(), "injured", "t1").await;

    // ── Essai : crédite du SPP et incrémente le compteur de carrière ──────────
    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(context("scorer")).to_enveloppe(),
    );
    wait_for(|| {
        let player_repo = player_repo.clone();
        async move {
            player_repo
                .find_by_id(&PlayerId("scorer".into()))
                .await
                .ok()
                .flatten()
                .map(|p| p.spp.0 > 0)
                .unwrap_or(false)
        }
    })
    .await;
    let scorer = player_repo.find_by_id(&PlayerId("scorer".into())).await.unwrap().unwrap();
    assert_eq!(scorer.spp.0, 3);
    assert_eq!(scorer.career_touchdowns.0, 1);

    // ── Blessure sérieuse : passe le joueur en MissingNextGame ────────────────
    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::PlayerInjured {
            context:     context("injured"),
            injury_type: InjuryTypePayload::BlessureSerieuse,
        }
        .to_enveloppe(),
    );
    wait_for(|| {
        let player_repo = player_repo.clone();
        async move {
            player_repo
                .find_by_id(&PlayerId("injured".into()))
                .await
                .ok()
                .flatten()
                .map(|p| p.participation_status == PlayerParticipationStatus::MissingNextGame)
                .unwrap_or(false)
        }
    })
    .await;

    // ── TeamMatchConcluded : restaure la disponibilité du joueur blessé ───────
    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::TeamMatchConcluded {
            team_id:         "t1".into(),
            match_report_id: "mr2".into(),
        }
        .to_enveloppe(),
    );
    wait_for(|| {
        let player_repo = player_repo.clone();
        async move {
            player_repo
                .find_by_id(&PlayerId("injured".into()))
                .await
                .ok()
                .flatten()
                .map(|p| p.participation_status == PlayerParticipationStatus::Available)
                .unwrap_or(false)
        }
    })
    .await;

    let injured = player_repo.find_by_id(&PlayerId("injured".into())).await.unwrap().unwrap();
    assert_eq!(injured.participation_status, PlayerParticipationStatus::Available);
    assert_eq!(injured.career_persistent_injuries.0, 1); // conservé après restauration
}
