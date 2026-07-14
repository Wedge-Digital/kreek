use crate::app::players::domain::match_impact::{MatchReportId, PlayerParticipationStatus};
use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::IPlayerRepository;
use crate::app::shared_kernel::app_events::player_match_impact_app_events::PlayerMatchImpactAppEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Lève `MissingNextGame` → `Available` pour tous les joueurs d'une équipe dont
/// le rapport de match vient de se conclure (BR12) — no-op pour les autres statuts.
pub fn init(app_event_bus: &EventBus, player_repo: Arc<dyn IPlayerRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<PlayerMatchImpactAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let PlayerMatchImpactAppEvent::TeamMatchConcluded { team_id, match_report_id } = app_event
                    else {
                        continue;
                    };
                    handle_team_match_concluded(player_repo.as_ref(), &team_id, &match_report_id).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("team_match_concluded_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_team_match_concluded(
    player_repo: &dyn IPlayerRepository,
    team_id: &str,
    match_report_id: &str,
) {
    let players = match player_repo.find_by_team_id(&TeamId(team_id.to_string())).await {
        Ok(players) => players,
        Err(e) => {
            tracing::error!("team_match_concluded_listener: find_by_team_id {team_id}: {e}");
            return;
        }
    };

    for player in players.iter().filter(|p| p.participation_status == PlayerParticipationStatus::MissingNextGame) {
        let event = player.restore_availability(MatchReportId(match_report_id.to_string()));
        let next_version = player.version + 1;
        if let Err(e) = player_repo.append(&player.id, &player.team_id, &event, next_version).await {
            tracing::error!(
                "team_match_concluded_listener: append {}: {e}",
                player.id.0
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::InjuryType;
    use crate::app::players::domain::match_impact::MatchContext;
    use crate::app::players::domain::match_impact::RoundId as MatchImpactRoundId;
    use crate::app::players::domain::player::{PlayerId, Spp, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
    use crate::app::players::io::repository::player_repository::PgPlayerRepository;
    use crate::app::shared_kernel::common_types::SpaceId;
    use sqlx::PgPool;

    fn sample_context() -> MatchContext {
        MatchContext {
            match_report_id:    MatchReportId("mr1".into()),
            round_id:           MatchImpactRoundId("r1".into()),
            round_label:        "Journée 5".into(),
            opponent_team_id:   TeamId("opponent".into()),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    async fn seed_player(repo: &PgPlayerRepository, player_id: &str, team_id: &str) -> crate::app::players::domain::player::Player {
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
        crate::app::players::domain::player::Player::from_events(&[created]).unwrap()
    }

    #[sqlx::test]
    async fn restores_only_missing_next_game_players_of_the_team(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);

        let injured = seed_player(&repo, "injured", "t1").await;
        let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&injured.id, &injured.team_id, &injury_event, 2).await.unwrap();

        seed_player(&repo, "healthy", "t1").await;
        seed_player(&repo, "other_team_injured", "t2").await;

        handle_team_match_concluded(&repo, "t1", "mr2").await;

        let injured_after = repo.find_by_id(&PlayerId("injured".into())).await.unwrap().unwrap();
        assert_eq!(injured_after.participation_status, PlayerParticipationStatus::Available);

        let healthy_after = repo.find_by_id(&PlayerId("healthy".into())).await.unwrap().unwrap();
        assert_eq!(healthy_after.version, 1); // no-op, jamais touché
    }
}
