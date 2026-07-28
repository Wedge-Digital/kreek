#![cfg(test)]

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{InjuryType, MatchContext, MatchReportId, RoundId, SppEarned};
use crate::app::players::domain::player::{Player, PlayerId, TeamId, Spp, ValueKpo};
use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
use crate::app::players::ports::{IPlayerProjectionRepository, IPlayerRepository};
use crate::app::shared_kernel::identity::ids::SpaceId;
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

#[sqlx::test]
async fn append_match_concluded_increments_matches_played_and_projection_version(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let player_id = PlayerId("p4".into());
    let team_id = TeamId("t1".into());
    let player = seed_player(&repo, &player_id, &team_id).await;

    let event = player.record_match_concluded(sample_context(), 2, 1);
    repo.append(&player_id, &team_id, &event, 2).await.unwrap();

    let reloaded = repo.find_by_id(&player_id).await.unwrap().unwrap();
    assert_eq!(reloaded.matches_played.0, 1);
    assert_eq!(reloaded.version, 2);
}

#[sqlx::test]
async fn find_events_by_id_returns_raw_events_in_order(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let player_id = PlayerId("p5".into());
    let team_id = TeamId("t1".into());
    let player = seed_player(&repo, &player_id, &team_id).await;

    let touchdown = player.record_touchdown(sample_context(), SppEarned::try_new(3).unwrap());
    repo.append(&player_id, &team_id, &touchdown, 2).await.unwrap();
    let player = Player::from_events(&repo.find_events_by_id(&player_id).await.unwrap()).unwrap();
    let concluded = player.record_match_concluded(sample_context(), 2, 1);
    repo.append(&player_id, &team_id, &concluded, 3).await.unwrap();

    let events = repo.find_events_by_id(&player_id).await.unwrap();
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], PlayerDomainEvent::PlayerCreated { .. }));
    assert!(matches!(events[1], PlayerDomainEvent::TouchdownScored { .. }));
    assert!(matches!(events[2], PlayerDomainEvent::MatchConcluded { .. }));
}

// ── has_spent_spp_since_match — garde-fou de correction ───────────────────────

fn context_for(match_report_id: &str) -> MatchContext {
    MatchContext {
        match_report_id:    MatchReportId(match_report_id.into()),
        round_id:           RoundId("r1".into()),
        round_label:        "Journée 5".into(),
        opponent_team_id:   TeamId("opponent".into()),
        opponent_team_name: "Bone Crushers".into(),
    }
}

/// Épingle le couplage entre la requête SQL et la représentation serde de
/// `PlayerDomainEvent`. La requête navigue le payload par
/// `payload -> 'MatchConcluded' -> 'context' ->> 'match_report_id'` : si l'enum
/// gagnait un jour un `#[serde(tag = ...)]`, le SQL cesserait de trouver quoi
/// que ce soit **sans erreur**, et le garde-fou laisserait tout passer.
#[test]
fn la_forme_json_de_match_concluded_expose_le_match_report_id() {
    let event = PlayerDomainEvent::MatchConcluded {
        player_id:      PlayerId("p1".into()),
        team_id:        TeamId("t1".into()),
        context:        context_for("mr-42"),
        team_score:     2,
        opponent_score: 1,
    };

    let payload = serde_json::to_value(&event).unwrap();

    assert_eq!(
        payload["MatchConcluded"]["context"]["match_report_id"],
        serde_json::json!("mr-42"),
        "la requête SQL de has_spent_spp_since_match dépend de ce chemin exact"
    );
}

/// `seed_player` crée un joueur à 0 SPP, ce qui ferait refuser tout achat par le
/// domaine. Ces tests ont besoin d'un pool dépensable.
async fn seed_player_with_spp(
    repo:      &PgPlayerRepository,
    player_id: &PlayerId,
    team_id:   &TeamId,
) -> Player {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id:      player_id.clone(),
        team_id:        team_id.clone(),
        space_id:       SpaceId::new(),
        position_name:  PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey:         None,
        base_skills:    vec![],
        starting_spp:   Spp(20),
        starting_value: ValueKpo(100_000),
    };
    repo.append(player_id, team_id, &created, 1).await.unwrap();
    Player::from_events(&[created]).unwrap()
}

async fn purchase_skill(
    repo:      &PgPlayerRepository,
    player:    &Player,
    version:   i32,
) {
    use crate::app::players::domain::player::AcquisitionMode;
    use crate::app::players::domain::value_objects::{SkillId, SkillName, SppCost};
    let event = player
        .purchase_skill(
            SkillId::try_new("BLOCK".to_string()).unwrap(),
            SkillName::try_new("Blocage".to_string()).unwrap(),
            "general".to_string(),
            AcquisitionMode::Chosen,
            SppCost::try_new(3).unwrap(),
            ValueKpo(20_000),
        )
        .unwrap();
    repo.append(&player.id, &player.team_id, &event, version).await.unwrap();
}

#[sqlx::test]
async fn has_spent_spp_since_match_est_faux_sans_achat(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player_with_spp(&repo, &player_id, &team_id).await;

    let concluded = player.record_match_concluded(context_for("mr-1"), 1, 0);
    repo.append(&player_id, &team_id, &concluded, 2).await.unwrap();

    assert!(!repo.has_spent_spp_since_match(&team_id, "mr-1").await.unwrap());
}

#[sqlx::test]
async fn has_spent_spp_since_match_est_vrai_apres_un_achat(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player_with_spp(&repo, &player_id, &team_id).await;

    let concluded = player.record_match_concluded(context_for("mr-1"), 1, 0);
    repo.append(&player_id, &team_id, &concluded, 2).await.unwrap();
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    purchase_skill(&repo, &player, 3).await;

    assert!(repo.has_spent_spp_since_match(&team_id, "mr-1").await.unwrap());
}

/// Le test décisif : un achat **antérieur** au match ne doit pas bloquer sa
/// correction. C'est ce qui justifie la sous-requête sur le match demandé,
/// plutôt qu'un simple « cette équipe a-t-elle déjà dépensé des SPP ».
#[sqlx::test]
async fn un_achat_anterieur_au_match_ne_bloque_pas_sa_correction(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player_with_spp(&repo, &player_id, &team_id).await;

    // achat après un match précédent…
    let previous = player.record_match_concluded(context_for("mr-0"), 1, 0);
    repo.append(&player_id, &team_id, &previous, 2).await.unwrap();
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    purchase_skill(&repo, &player, 3).await;

    // …puis le match qu'on veut corriger, sans achat depuis
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    let current = player.record_match_concluded(context_for("mr-1"), 2, 2);
    repo.append(&player_id, &team_id, &current, 4).await.unwrap();

    assert!(!repo.has_spent_spp_since_match(&team_id, "mr-1").await.unwrap());
    // le match précédent, lui, n'est plus corrigeable
    assert!(repo.has_spent_spp_since_match(&team_id, "mr-0").await.unwrap());
}

#[sqlx::test]
async fn l_achat_d_une_autre_equipe_ne_compte_pas(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (mine, other) = (TeamId("t1".into()), TeamId("t2".into()));
    let my_player = seed_player_with_spp(&repo, &PlayerId("p1".into()), &mine).await;
    let their_player = seed_player_with_spp(&repo, &PlayerId("p2".into()), &other).await;

    for (player, team) in [(&my_player, &mine), (&their_player, &other)] {
        let concluded = player.record_match_concluded(context_for("mr-1"), 1, 0);
        repo.append(&player.id, team, &concluded, 2).await.unwrap();
    }
    let their_player = repo.find_by_id(&PlayerId("p2".into())).await.unwrap().unwrap();
    purchase_skill(&repo, &their_player, 3).await;

    assert!(!repo.has_spent_spp_since_match(&mine, "mr-1").await.unwrap());
    assert!(repo.has_spent_spp_since_match(&other, "mr-1").await.unwrap());
}

#[sqlx::test]
async fn un_match_inconnu_ne_bloque_pas(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    seed_player_with_spp(&repo, &player_id, &team_id).await;

    assert!(!repo.has_spent_spp_since_match(&team_id, "mr-inconnu").await.unwrap());
}

// ── MatchImpactReverted — projection ─────────────────────────────────────────

/// La projection doit suivre l'agrégat après compensation. L'événement étant
/// mince, `upsert_player_projection` relit le flux dans sa transaction : ce test
/// vérifie que le résultat est bien celui de l'agrégat rejoué.
#[sqlx::test]
async fn la_compensation_met_a_jour_spp_et_statut_dans_la_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player(&repo, &player_id, &team_id).await;

    let td = player.record_touchdown(context_for("mr-1"), SppEarned::try_new(3).unwrap());
    repo.append(&player_id, &team_id, &td, 2).await.unwrap();
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    let blessure = player.record_injury(context_for("mr-1"), InjuryType::Amoche);
    repo.append(&player_id, &team_id, &blessure, 3).await.unwrap();

    let avant = proj.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(avant.spp, 3);
    assert_eq!(avant.participation_status, "MissingNextGame");

    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    let compensation = player
        .revert_match_impact(&MatchReportId("mr-1".into()))
        .expect("le dernier match doit être compensable");
    repo.append(&player_id, &team_id, &compensation, 4).await.unwrap();

    let apres = proj.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(apres.spp, 0, "les SPP du match doivent être retirés de la projection");
    assert_eq!(
        apres.participation_status, "Available",
        "le statut projeté doit suivre l'agrégat"
    );
}
