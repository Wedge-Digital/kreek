#![cfg(test)]

use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::{
    InjuryType, MatchContext, MatchReportId, RoundId, SppEarned, StatKind,
};
use crate::app::players::domain::player::{Player, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{
    JerseyVo, PositionNameVo, RosterLineId, SkillId, SkillName,
};
use crate::app::players::io::repository::player_repository::PgPlayerRepository;
use crate::app::players::io::repository::projection_repository::PgPlayerProjectionRepository;
use crate::app::players::ports::{IPlayerProjectionRepository, IPlayerRepository};
use crate::app::shared_kernel::identity::ids::SpaceId;
use sqlx::PgPool;

fn sample_context() -> MatchContext {
    MatchContext {
        match_report_id: MatchReportId("mr1".into()),
        round_id: RoundId("r1".into()),
        round_label: "Journée 5".into(),
        opponent_team_id: TeamId("opponent".into()),
        opponent_team_name: "Bone Crushers".into(),
    }
}

async fn seed_player(repo: &PgPlayerRepository, player_id: &PlayerId, team_id: &TeamId) -> Player {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id: player_id.clone(),
        team_id: team_id.clone(),
        space_id: SpaceId::new(),
        position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey: None,
        base_skills: vec![],
        starting_spp: Spp(0),
        starting_value: ValueKpo(100),
    };
    repo.append(player_id, team_id, &created, 1).await.unwrap();
    Player::from_events(&[created]).unwrap()
}

async fn seed_player_with_jersey(
    repo: &PgPlayerRepository,
    player_id: &PlayerId,
    team_id: &TeamId,
    jersey: u16,
) {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id: player_id.clone(),
        team_id: team_id.clone(),
        space_id: SpaceId::new(),
        position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey: Some(JerseyVo::try_new(jersey).unwrap()),
        base_skills: vec![],
        starting_spp: Spp(0),
        starting_value: ValueKpo(100),
    };
    repo.append(player_id, team_id, &created, 1).await.unwrap();
}

// ── Appartenance à l'effectif (carte 260) ────────────────────────────────────

/// Un renvoyé sort de l'effectif sans être effacé : c'est toute la différence
/// entre `find_by_team_id` et `find_by_id`, et c'est ce qui permet à ses SPP et
/// à son historique de survivre.
#[sqlx::test]
async fn un_renvoye_quitte_l_effectif_mais_reste_lisible(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-renvois".into());
    let renvoye = PlayerId("parti".into());
    let reste = PlayerId("reste".into());

    seed_player(&repo, &renvoye, &team_id).await;
    seed_player(&repo, &reste, &team_id).await;
    assert_eq!(repo.find_by_team_id(&team_id).await.unwrap().len(), 2);

    let renvoi = PlayerDomainEvent::PlayerDismissed {
        player_id: renvoye.clone(),
        team_id: team_id.clone(),
    };
    repo.append(&renvoye, &team_id, &renvoi, 2).await.unwrap();

    let effectif = repo.find_by_team_id(&team_id).await.unwrap();
    assert_eq!(effectif.len(), 1, "le renvoyé a quitté l'effectif");
    assert_eq!(effectif[0].id, reste);

    let toujours_la = repo.find_by_id(&renvoye).await.unwrap().unwrap();
    assert!(!toujours_la.membership.is_active());
    assert_eq!(toujours_la.value, ValueKpo(100), "il garde sa valeur");
}

#[sqlx::test]
async fn un_renvoye_disparait_aussi_de_la_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool);
    let team_id = TeamId("t-proj-renvois".into());
    let renvoye = PlayerId("parti-proj".into());

    seed_player(&repo, &renvoye, &team_id).await;
    let renvoi = PlayerDomainEvent::PlayerDismissed {
        player_id: renvoye.clone(),
        team_id: team_id.clone(),
    };
    repo.append(&renvoye, &team_id, &renvoi, 2).await.unwrap();

    assert!(proj.find_by_team_id(&team_id).await.unwrap().is_empty());
    assert_eq!(proj.count_available_by_team_id(&team_id).await.unwrap(), 0);
    // Non effacé pour autant : la fiche du joueur reste consultable.
    assert!(proj.find_by_id(&renvoye.0).await.unwrap().is_some());
}

/// La promesse de la carte 265, enfin vraie : le numéro d'un renvoyé redevient
/// attribuable. Elle ne l'était pas tant que la recherche de maillot lisait
/// `players_proj` par elle-même.
#[sqlx::test]
async fn le_maillot_d_un_renvoye_redevient_attribuable(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool);
    let team_id = TeamId("t-maillots".into());
    let porteur = PlayerId("porteur-du-3".into());

    seed_player_with_jersey(&repo, &PlayerId("un".into()), &team_id, 1).await;
    seed_player_with_jersey(&repo, &PlayerId("deux".into()), &team_id, 2).await;
    seed_player_with_jersey(&repo, &porteur, &team_id, 3).await;

    let mut pris = proj.jerseys_by_team_id(&team_id).await.unwrap();
    pris.sort_unstable();
    assert_eq!(pris, vec![1, 2, 3]);

    let renvoi = PlayerDomainEvent::PlayerDismissed {
        player_id: porteur.clone(),
        team_id: team_id.clone(),
    };
    repo.append(&porteur, &team_id, &renvoi, 2).await.unwrap();

    let mut apres = proj.jerseys_by_team_id(&team_id).await.unwrap();
    apres.sort_unstable();
    assert_eq!(apres, vec![1, 2], "le 3 est libéré");
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
    repo.append(&injured_id, &team_id, &injury_event, 2)
        .await
        .unwrap();

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
    repo.append(&injured_id, &team_id, &restore_event, 3)
        .await
        .unwrap();

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
    repo.append(&player_id, &team_id, &touchdown, 2)
        .await
        .unwrap();
    let player = Player::from_events(&repo.find_events_by_id(&player_id).await.unwrap()).unwrap();
    let concluded = player.record_match_concluded(sample_context(), 2, 1);
    repo.append(&player_id, &team_id, &concluded, 3)
        .await
        .unwrap();

    let events = repo.find_events_by_id(&player_id).await.unwrap();
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], PlayerDomainEvent::PlayerCreated { .. }));
    assert!(matches!(
        events[1],
        PlayerDomainEvent::TouchdownScored { .. }
    ));
    assert!(matches!(
        events[2],
        PlayerDomainEvent::MatchConcluded { .. }
    ));
}

// ── has_spent_spp_since_match — garde-fou de correction ───────────────────────

fn context_for(match_report_id: &str) -> MatchContext {
    MatchContext {
        match_report_id: MatchReportId(match_report_id.into()),
        round_id: RoundId("r1".into()),
        round_label: "Journée 5".into(),
        opponent_team_id: TeamId("opponent".into()),
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
        player_id: PlayerId("p1".into()),
        team_id: TeamId("t1".into()),
        context: context_for("mr-42"),
        team_score: 2,
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
    repo: &PgPlayerRepository,
    player_id: &PlayerId,
    team_id: &TeamId,
) -> Player {
    let created = PlayerDomainEvent::PlayerCreated {
        player_id: player_id.clone(),
        team_id: team_id.clone(),
        space_id: SpaceId::new(),
        position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
        roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
        jersey: None,
        base_skills: vec![],
        starting_spp: Spp(20),
        starting_value: ValueKpo(100),
    };
    repo.append(player_id, team_id, &created, 1).await.unwrap();
    Player::from_events(&[created]).unwrap()
}

async fn purchase_skill(repo: &PgPlayerRepository, player: &Player, version: i32) {
    use crate::app::players::domain::player::AcquisitionMode;
    use crate::app::players::domain::value_objects::{SkillId, SkillName, SppCost};
    let event = player
        .purchase_skill(
            SkillId::try_new("BLOCK".to_string()).unwrap(),
            SkillName::try_new("Blocage".to_string()).unwrap(),
            "general".to_string(),
            AcquisitionMode::Chosen,
            SppCost::try_new(3).unwrap(),
            ValueKpo(20),
        )
        .unwrap();
    repo.append(&player.id, &player.team_id, &event, version)
        .await
        .unwrap();
}

#[sqlx::test]
async fn has_spent_spp_since_match_est_faux_sans_achat(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player_with_spp(&repo, &player_id, &team_id).await;

    let concluded = player.record_match_concluded(context_for("mr-1"), 1, 0);
    repo.append(&player_id, &team_id, &concluded, 2)
        .await
        .unwrap();

    assert!(!repo
        .has_spent_spp_since_match(&team_id, "mr-1")
        .await
        .unwrap());
}

#[sqlx::test]
async fn has_spent_spp_since_match_est_vrai_apres_un_achat(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    let player = seed_player_with_spp(&repo, &player_id, &team_id).await;

    let concluded = player.record_match_concluded(context_for("mr-1"), 1, 0);
    repo.append(&player_id, &team_id, &concluded, 2)
        .await
        .unwrap();
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    purchase_skill(&repo, &player, 3).await;

    assert!(repo
        .has_spent_spp_since_match(&team_id, "mr-1")
        .await
        .unwrap());
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
    repo.append(&player_id, &team_id, &previous, 2)
        .await
        .unwrap();
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    purchase_skill(&repo, &player, 3).await;

    // …puis le match qu'on veut corriger, sans achat depuis
    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    let current = player.record_match_concluded(context_for("mr-1"), 2, 2);
    repo.append(&player_id, &team_id, &current, 4)
        .await
        .unwrap();

    assert!(!repo
        .has_spent_spp_since_match(&team_id, "mr-1")
        .await
        .unwrap());
    // le match précédent, lui, n'est plus corrigeable
    assert!(repo
        .has_spent_spp_since_match(&team_id, "mr-0")
        .await
        .unwrap());
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
    let their_player = repo
        .find_by_id(&PlayerId("p2".into()))
        .await
        .unwrap()
        .unwrap();
    purchase_skill(&repo, &their_player, 3).await;

    assert!(!repo.has_spent_spp_since_match(&mine, "mr-1").await.unwrap());
    assert!(repo
        .has_spent_spp_since_match(&other, "mr-1")
        .await
        .unwrap());
}

#[sqlx::test]
async fn un_match_inconnu_ne_bloque_pas(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool);
    let (player_id, team_id) = (PlayerId("p1".into()), TeamId("t1".into()));
    seed_player_with_spp(&repo, &player_id, &team_id).await;

    assert!(!repo
        .has_spent_spp_since_match(&team_id, "mr-inconnu")
        .await
        .unwrap());
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
    repo.append(&player_id, &team_id, &blessure, 3)
        .await
        .unwrap();

    let avant = proj.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(avant.spp, 3);
    assert_eq!(avant.participation_status, "MissingNextGame");

    let player = repo.find_by_id(&player_id).await.unwrap().unwrap();
    let compensation = player
        .revert_match_impact(&MatchReportId("mr-1".into()))
        .expect("le dernier match doit être compensable");
    repo.append(&player_id, &team_id, &compensation, 4)
        .await
        .unwrap();

    let apres = proj.find_by_id(&player_id.0).await.unwrap().unwrap();
    assert_eq!(
        apres.spp, 0,
        "les SPP du match doivent être retirés de la projection"
    );
    assert_eq!(
        apres.participation_status, "Available",
        "le statut projeté doit suivre l'agrégat"
    );
}

/// Le comptage qui détermine le nombre de journaliers. Le confondre avec
/// l'effectif total prive de renfort une équipe amoindrie : avec 13 joueurs
/// dont 4 blessés, `11 - 13` donne zéro journalier alors qu'il en faut deux.
#[sqlx::test]
async fn count_available_by_team_id_exclut_les_indisponibles(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool);
    let team_id = TeamId("t-blesses".into());

    for i in 0..13 {
        let player_id = PlayerId(format!("p{i}"));
        let player = seed_player(&repo, &player_id, &team_id).await;
        if i < 4 {
            let injury = player.record_injury(sample_context(), InjuryType::BlessureSerieuse);
            repo.append(&player_id, &team_id, &injury, 2).await.unwrap();
        }
    }

    let total = proj.find_by_team_id(&team_id).await.unwrap().len();
    let disponibles = proj.count_available_by_team_id(&team_id).await.unwrap();

    assert_eq!(total, 13, "l'effectif total reste de 13");
    assert_eq!(disponibles, 9, "4 blessés ne sont pas alignables");
    assert_eq!(
        11usize.saturating_sub(disponibles),
        2,
        "l'équipe doit recevoir 2 journaliers"
    );
}

// ── Édition de l'effectif (carte 291) ────────────────────────────────────────

/// `append_batch` doit committer tout le lot d'un coup, même quand il touche
/// plusieurs joueurs : c'est ce qui empêche un doublon de maillot d'exister le
/// temps d'un échec partiel.
#[sqlx::test]
async fn append_batch_persiste_plusieurs_joueurs_en_une_transaction(pool: PgPool) {
    use crate::app::players::domain::value_objects::{DisplayOrder, PersonalName};

    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool.clone());
    let team_id = TeamId("t-batch".into());
    let un = PlayerId("un".into());
    let deux = PlayerId("deux".into());

    seed_player_with_jersey(&repo, &un, &team_id, 1).await;
    seed_player_with_jersey(&repo, &deux, &team_id, 2).await;

    repo.append_batch(vec![
        (
            un.clone(),
            team_id.clone(),
            PlayerDomainEvent::PlayerRenamed {
                player_id: un.clone(),
                team_id: team_id.clone(),
                personal_name: Some(PersonalName::try_new("Grok".to_string()).unwrap()),
            },
            2,
        ),
        (
            deux.clone(),
            team_id.clone(),
            PlayerDomainEvent::PlayerJerseyChanged {
                player_id: deux.clone(),
                team_id: team_id.clone(),
                jersey: Some(JerseyVo::try_new(42).unwrap()),
            },
            2,
        ),
    ])
    .await
    .unwrap();

    let effectif = proj.find_by_team_id(&team_id).await.unwrap();
    let ligne_un = effectif.iter().find(|p| p.player_id == un.0).unwrap();
    let ligne_deux = effectif.iter().find(|p| p.player_id == deux.0).unwrap();

    assert_eq!(ligne_un.personal_name, "Grok");
    assert_eq!(ligne_deux.jersey, Some(42));

    // L'agrégat se rejoue : l'événement est bien dans l'event store, pas
    // seulement dans la projection.
    let recharge = repo.find_by_id(&un).await.unwrap().unwrap();
    assert_eq!(recharge.personal_name.unwrap().as_ref(), "Grok");

    // L'effacement : le domaine dit `None`, la colonne est NOT NULL, la
    // projection doit donc y écrire `''` — et non planter sur la contrainte.
    repo.append_batch(vec![(
        un.clone(),
        team_id.clone(),
        PlayerDomainEvent::PlayerRenamed {
            player_id: un.clone(),
            team_id: team_id.clone(),
            personal_name: None,
        },
        3,
    )])
    .await
    .unwrap();
    let effectif = proj.find_by_team_id(&team_id).await.unwrap();
    let ligne_un = effectif.iter().find(|p| p.player_id == un.0).unwrap();
    assert_eq!(ligne_un.personal_name, "");
    assert!(repo
        .find_by_id(&un)
        .await
        .unwrap()
        .unwrap()
        .personal_name
        .is_none());

    // Et l'ordre libre suit le même chemin.
    repo.append_batch(vec![(
        deux.clone(),
        team_id.clone(),
        PlayerDomainEvent::PlayerReordered {
            player_id: deux.clone(),
            team_id: team_id.clone(),
            display_order: DisplayOrder::new(7),
        },
        3,
    )])
    .await
    .unwrap();
    let recharge = repo.find_by_id(&deux).await.unwrap().unwrap();
    assert_eq!(recharge.display_order.unwrap().into_inner(), 7);
}

/// Le tri de l'effectif : l'ordre posé par le coach prime, et un joueur jamais
/// réordonné passe derrière — quel que soit son numéro de maillot.
#[sqlx::test]
async fn un_joueur_ordonne_passe_avant_un_joueur_sans_ordre(pool: PgPool) {
    use crate::app::players::domain::value_objects::DisplayOrder;

    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool.clone());
    let team_id = TeamId("t-tri".into());
    let sans_ordre = PlayerId("sans".into());
    let avec_ordre = PlayerId("avec".into());

    // `sans_ordre` porte le plus petit maillot : sans la nouvelle clé de tri,
    // c'est lui qui sortirait en tête.
    seed_player_with_jersey(&repo, &sans_ordre, &team_id, 1).await;
    seed_player_with_jersey(&repo, &avec_ordre, &team_id, 2).await;

    repo.append_batch(vec![(
        avec_ordre.clone(),
        team_id.clone(),
        PlayerDomainEvent::PlayerReordered {
            player_id: avec_ordre.clone(),
            team_id: team_id.clone(),
            display_order: DisplayOrder::new(0),
        },
        2,
    )])
    .await
    .unwrap();

    let effectif = proj.find_by_team_id(&team_id).await.unwrap();
    let ordre: Vec<&str> = effectif.iter().map(|p| p.player_id.as_str()).collect();
    assert_eq!(
        ordre,
        vec!["avec", "sans"],
        "le joueur réordonné doit précéder celui qui n'a pas d'ordre"
    );
}

// ── Deltas de caractéristiques en projection (carte 303) ─────────────────────

use crate::app::players::domain::value_objects::{
    CustomisationId, KpoDelta, SppAmount, SppCost, StatCrans,
};

async fn deltas(pool: &PgPool, player_id: &str) -> (i16, i16, i16, i16, i16) {
    sqlx::query_as(
        "SELECT ma_delta, st_delta, ag_delta, pa_delta, av_delta
         FROM players_proj WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Construit l'événement d'augmentation **sans passer par la garde de solde** :
/// ces tests portent sur la projection, et exiger un solde suffisant y
/// ajouterait une précondition sans rapport avec ce qu'ils vérifient.
fn augmentation(joueur: &PlayerId, team_id: &TeamId, stat: StatKind) -> PlayerDomainEvent {
    PlayerDomainEvent::PlayerStatIncreased {
        player_id: joueur.clone(),
        team_id: team_id.clone(),
        stat,
        spp_cost: SppCost::try_new(6).unwrap(),
        value_delta: ValueKpo(0),
    }
}

fn custo_id(v: &str) -> CustomisationId {
    CustomisationId::try_new(v.to_string()).unwrap()
}

/// Une augmentation SPP écrit son cran dans le sens de la caractéristique :
/// MV monte, AG descend.
#[sqlx::test]
async fn une_augmentation_spp_ecrit_le_delta_dans_le_bon_sens(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-delta".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    let event = augmentation(&joueur, &team_id, StatKind::Ma);
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();
    assert_eq!(deltas(&pool, &joueur.0).await.0, 1, "MV monte");

    let p = repo.find_by_id(&joueur).await.unwrap().unwrap();
    let event = augmentation(&joueur, &team_id, StatKind::Ag);
    repo.append(&joueur, &team_id, &event, 3).await.unwrap();
    assert_eq!(deltas(&pool, &joueur.0).await.2, -1, "AG descend");
}

/// Une séquelle est l'exact inverse d'un cran d'amélioration.
#[sqlx::test]
async fn une_sequelle_ecrit_un_delta_negatif(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-sequelle".into());
    let joueur = PlayerId("p".into());
    seed_player(&repo, &joueur, &team_id).await;

    let event = PlayerDomainEvent::InjurySustained {
        player_id: joueur.clone(),
        team_id: team_id.clone(),
        context: sample_context(),
        injury_type: InjuryType::Sequel { stat: StatKind::Ma },
    };
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();

    // MV perd un point : le malus va dans le sens inverse de l'amélioration.
    assert_eq!(deltas(&pool, &joueur.0).await.0, -1);
}

/// **Le test qui justifie le recalcul.** Une compensation de match ramène le
/// delta à sa valeur d'avant, alors que l'événement ne porte aucun montant à
/// retrancher — il est mince à dessein.
#[sqlx::test]
async fn une_compensation_de_match_ramene_le_delta_a_sa_valeur_d_avant(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-revert".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    // Une augmentation achetée avant le match : elle doit survivre à la
    // compensation, contrairement à la séquelle.
    let event = augmentation(&joueur, &team_id, StatKind::St);
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();
    assert_eq!(deltas(&pool, &joueur.0).await.1, 1);

    let event = PlayerDomainEvent::InjurySustained {
        player_id: joueur.clone(),
        team_id: team_id.clone(),
        context: sample_context(),
        injury_type: InjuryType::Sequel { stat: StatKind::St },
    };
    repo.append(&joueur, &team_id, &event, 3).await.unwrap();
    assert_eq!(
        deltas(&pool, &joueur.0).await.1,
        0,
        "la séquelle annule le cran"
    );

    let event = PlayerDomainEvent::MatchImpactReverted {
        player_id: joueur.clone(),
        team_id: team_id.clone(),
        match_report_id: MatchReportId("mr1".into()),
    };
    repo.append(&joueur, &team_id, &event, 4).await.unwrap();

    assert_eq!(
        deltas(&pool, &joueur.0).await.1,
        1,
        "la compensation défait la séquelle, pas l'augmentation"
    );
}

#[sqlx::test]
async fn une_customisation_de_caracteristique_ecrit_son_offset(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-custo".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    // Améliorer l'agilité de deux crans : offset brut de -2.
    let event = p
        .customise_stat(
            custo_id("c1"),
            StatKind::Ag,
            StatCrans::try_new(2).unwrap(),
            "Bagouze".into(),
        )
        .unwrap();
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();

    assert_eq!(deltas(&pool, &joueur.0).await.2, -2);
}

/// Les trois sources se cumulent sur la même caractéristique.
#[sqlx::test]
async fn les_trois_sources_se_cumulent(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-cumul".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    let event = augmentation(&joueur, &team_id, StatKind::Ma);
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();

    let event = PlayerDomainEvent::InjurySustained {
        player_id: joueur.clone(),
        team_id: team_id.clone(),
        context: sample_context(),
        injury_type: InjuryType::Sequel { stat: StatKind::Ma },
    };
    repo.append(&joueur, &team_id, &event, 3).await.unwrap();

    let p = repo.find_by_id(&joueur).await.unwrap().unwrap();
    let event = p
        .customise_stat(
            custo_id("c2"),
            StatKind::Ma,
            StatCrans::try_new(3).unwrap(),
            "Bagouze".into(),
        )
        .unwrap();
    repo.append(&joueur, &team_id, &event, 4).await.unwrap();

    // +1 (augmentation) -1 (séquelle) +3 (customisation) = +3
    assert_eq!(deltas(&pool, &joueur.0).await.0, 3);
}

/// Une compétence customisée rejoint les acquises **sans** déplacer la valeur —
/// c'est la règle la plus contre-intuitive de la fonctionnalité.
#[sqlx::test]
async fn une_competence_customisee_ne_deplace_pas_la_valeur(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool.clone());
    let team_id = TeamId("t-skill".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    let avant = proj.find_by_id(&joueur.0).await.unwrap().unwrap().value_kpo;

    let event = p
        .customise_skill(
            custo_id("c3"),
            SkillId::try_new("BLOCK".to_string()).unwrap(),
            SkillName::try_new("Bloc".to_string()).unwrap(),
            "Bagouze".into(),
        )
        .unwrap();
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();

    let apres = proj.find_by_id(&joueur.0).await.unwrap().unwrap();
    assert_eq!(apres.value_kpo, avant, "la valeur ne doit pas bouger");
    assert_eq!(apres.acquired_skills.len(), 1);
    assert_eq!(apres.acquired_skills[0].mode, "Customised");
}

/// Le prix, lui, la déplace — et ne peut pas passer sous zéro en projection.
#[sqlx::test]
async fn le_prix_customise_deplace_la_valeur_sans_passer_sous_zero(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool.clone());
    let team_id = TeamId("t-prix".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    let event = p
        .customise_value(
            custo_id("c4"),
            KpoDelta::try_new(-40).unwrap(),
            "Bagouze".into(),
        )
        .unwrap();
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();
    assert_eq!(
        proj.find_by_id(&joueur.0).await.unwrap().unwrap().value_kpo,
        60
    );

    let p = repo.find_by_id(&joueur).await.unwrap().unwrap();
    let event = p
        .customise_value(
            custo_id("c5"),
            KpoDelta::try_new(-500).unwrap(),
            "Bagouze".into(),
        )
        .unwrap();
    repo.append(&joueur, &team_id, &event, 3).await.unwrap();
    assert_eq!(
        proj.find_by_id(&joueur.0).await.unwrap().unwrap().value_kpo,
        0
    );
}

#[sqlx::test]
async fn les_spp_customises_s_ajoutent_en_projection(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let proj = PgPlayerProjectionRepository::new(pool.clone());
    let team_id = TeamId("t-spp".into());
    let joueur = PlayerId("p".into());
    let p = seed_player(&repo, &joueur, &team_id).await;

    let event = p
        .customise_spp(custo_id("c6"), SppAmount::try_new(15).unwrap(), "B".into())
        .unwrap();
    repo.append(&joueur, &team_id, &event, 2).await.unwrap();

    assert_eq!(proj.find_by_id(&joueur.0).await.unwrap().unwrap().spp, 15);
}

/// Le recalcul rend la projection insensible à l'ordre : appendre les mêmes
/// événements dans un ordre différent donne le même cumul.
#[sqlx::test]
async fn le_cumul_est_insensible_a_l_ordre(pool: PgPool) {
    let repo = PgPlayerRepository::new(pool.clone());
    let team_id = TeamId("t-ordre".into());

    let a = PlayerId("a".into());
    seed_player(&repo, &a, &team_id).await;
    let e = augmentation(&a, &team_id, StatKind::Pa);
    repo.append(&a, &team_id, &e, 2).await.unwrap();
    let pa = repo.find_by_id(&a).await.unwrap().unwrap();
    let e = pa
        .customise_stat(
            custo_id("x"),
            StatKind::Pa,
            StatCrans::try_new(1).unwrap(),
            "B".into(),
        )
        .unwrap();
    repo.append(&a, &team_id, &e, 3).await.unwrap();

    let b = PlayerId("b".into());
    let pb = seed_player(&repo, &b, &team_id).await;
    let e = pb
        .customise_stat(
            custo_id("y"),
            StatKind::Pa,
            StatCrans::try_new(1).unwrap(),
            "B".into(),
        )
        .unwrap();
    repo.append(&b, &team_id, &e, 2).await.unwrap();
    let e = augmentation(&b, &team_id, StatKind::Pa);
    repo.append(&b, &team_id, &e, 3).await.unwrap();

    assert_eq!(deltas(&pool, &a.0).await.3, deltas(&pool, &b.0).await.3);
    assert_eq!(deltas(&pool, &a.0).await.3, -2);
}
