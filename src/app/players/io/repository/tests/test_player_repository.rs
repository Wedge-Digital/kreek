use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{InjuryType, MatchContext, MatchReportId, RoundId, SppEarned};
use crate::app::players::domain::player::{Player, PlayerId, TeamId, Spp, ValueKpo};
use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
use crate::app::players::ports::{IPlayerProjectionRepository, IPlayerRepository};
use crate::app::shared_kernel::common_types::SpaceId;
use sqlx::PgPool;

fn sample_context() -> MatchContext {
    MatchContext {
        match_report_id:    MatchReportId("mr1".into()),
        round_id:           RoundId("r1".into()),
        round_label:        "Journée 5".into(),
        opponent_team_id:   TeamId("opponent".into()),
        opponent_team_name: "Bone Crushers".into(),
    }
}

async fn seed_player(repo: &PgPlayerRepository, player_id: &PlayerId, team_id: &TeamId) -> Player {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id:      player_id.clone(),
        team_id:        team_id.clone(),
        space_id:       SpaceId::new(),
        position_name:  PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey:         None,
        base_skills:    vec![],
        starting_spp:   Spp(0),
        starting_value: ValueKpo(100_000),
    };
    repo.append(player_id, team_id, &created, 1).await.unwrap();
    Player::from_events(&[created]).unwrap()
}

#[sqlx::test]
async fn append_touchdown_scored_credits_spp_in_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj_repo = PgPlayerProjectionRepository::new(pool);
    let player_id = PlayerId("p1".into());
    let team_id = TeamId("t1".into());
    let player = seed_player(&repo, &player_id, &team_id).await;

    let event = player.record_touchdown(sample_context(), SppEarned::try_new(3).unwrap());
    repo.append(&player_id, &team_id, &event, 2).await.unwrap();

    let projection = proj_repo.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(projection.spp, 3);
}

#[sqlx::test]
async fn append_injury_sustained_updates_participation_status_in_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj_repo = PgPlayerProjectionRepository::new(pool);
    let player_id = PlayerId("p2".into());
    let team_id = TeamId("t1".into());
    let player = seed_player(&repo, &player_id, &team_id).await;

    let event = player.record_injury(sample_context(), InjuryType::BlessureSerieuse);
    repo.append(&player_id, &team_id, &event, 2).await.unwrap();

    let projection = proj_repo.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(projection.participation_status, "MissingNextGame");
}

#[sqlx::test]
async fn append_commotion_does_not_change_participation_status_in_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj_repo = PgPlayerProjectionRepository::new(pool);
    let player_id = PlayerId("p3".into());
    let team_id = TeamId("t1".into());
    let player = seed_player(&repo, &player_id, &team_id).await;

    let event = player.record_injury(sample_context(), InjuryType::Commotion);
    repo.append(&player_id, &team_id, &event, 2).await.unwrap();

    let projection = proj_repo.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(projection.participation_status, "Available");
}

#[sqlx::test]
async fn find_by_team_id_returns_only_missing_next_game_players_for_restoration(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t2".into());

    let injured_id = PlayerId("injured".into());
    let injured = seed_player(&repo, &injured_id, &team_id).await;
    let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
    repo.append(&injured_id, &team_id, &injury_event, 2).await.unwrap();

    let healthy_id = PlayerId("healthy".into());
    seed_player(&repo, &healthy_id, &team_id).await;

    let players = repo.find_by_team_id(&team_id).await.unwrap();
    assert_eq!(players.len(), 2);

    let missing_next_game: Vec<_> = players
        .iter()
        .filter(|p| {
            p.participation_status
                == crate::app::players::domain::match_impact::PlayerParticipationStatus::MissingNextGame
        })
        .collect();
    assert_eq!(missing_next_game.len(), 1);
    assert_eq!(missing_next_game[0].id, injured_id);

    let restore_event = missing_next_game[0].restore_availability(MatchReportId("mr2".into()));
    repo.append(&injured_id, &team_id, &restore_event, 3).await.unwrap();

    let players_after = repo.find_by_team_id(&team_id).await.unwrap();
    let still_missing = players_after
        .iter()
        .filter(|p| {
            p.participation_status
                == crate::app::players::domain::match_impact::PlayerParticipationStatus::MissingNextGame
        })
        .count();
    assert_eq!(still_missing, 0);
}
