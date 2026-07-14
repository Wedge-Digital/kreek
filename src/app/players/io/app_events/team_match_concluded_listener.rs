use crate::app::players::domain::match_impact::{MatchContext, PlayerParticipationStatus};
use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::IPlayerRepository;

/// Pour chaque joueur d'une équipe dont le rapport de match vient de se
/// conclure : enregistre toujours `MatchConcluded` (compteur de matchs joués +
/// ancre d'historique), et en plus lève `MissingNextGame` → `Available` (BR12)
/// pour ceux qui l'étaient — peu importe que le joueur ait vraiment joué ou non.
///
/// Appelé depuis `player_match_impact_listener` (même tâche séquentielle que les
/// events d'action) plutôt que depuis un listener à part : les deux catégories
/// d'events touchent le même agrégat joueur avec une version optimiste, et deux
/// tâches concurrentes se disputeraient la même version (l'une des deux perd la
/// course et son event est silencieusement abandonné).
pub(crate) async fn handle_team_match_concluded(
    player_repo: &dyn IPlayerRepository,
    team_id: &str,
    context: MatchContext,
    team_score: u8,
    opponent_score: u8,
) {
    let players = match player_repo.find_by_team_id(&TeamId(team_id.to_string())).await {
        Ok(players) => players,
        Err(e) => {
            tracing::error!("team_match_concluded_listener: find_by_team_id {team_id}: {e}");
            return;
        }
    };

    for player in &players {
        let concluded = player.record_match_concluded(context.clone(), team_score, opponent_score);
        let next_version = player.version + 1;
        if let Err(e) = player_repo.append(&player.id, &player.team_id, &concluded, next_version).await {
            tracing::error!("team_match_concluded_listener: append MatchConcluded {}: {e}", player.id.0);
            continue;
        }

        if player.participation_status == PlayerParticipationStatus::MissingNextGame {
            let restored = player.restore_availability(context.match_report_id.clone());
            if let Err(e) = player_repo.append(&player.id, &player.team_id, &restored, next_version + 1).await {
                tracing::error!("team_match_concluded_listener: append PlayerAvailabilityRestored {}: {e}", player.id.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::InjuryType;
    use crate::app::players::domain::match_impact::MatchContext;
    use crate::app::players::domain::match_impact::MatchReportId;
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

    fn concluded_context() -> MatchContext {
        MatchContext {
            match_report_id:    MatchReportId("mr2".into()),
            round_id:           MatchImpactRoundId("r2".into()),
            round_label:        "Journée 6".into(),
            opponent_team_id:   TeamId("opponent2".into()),
            opponent_team_name: "Green Machine".into(),
        }
    }

    #[sqlx::test]
    async fn restores_only_missing_next_game_players_of_the_team(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);

        let injured = seed_player(&repo, "injured", "t1").await;
        let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&injured.id, &injured.team_id, &injury_event, 2).await.unwrap();

        seed_player(&repo, "healthy", "t1").await;
        seed_player(&repo, "other_team_injured", "t2").await;

        handle_team_match_concluded(&repo, "t1", concluded_context(), 2, 1).await;

        let injured_after = repo.find_by_id(&PlayerId("injured".into())).await.unwrap().unwrap();
        assert_eq!(injured_after.participation_status, PlayerParticipationStatus::Available);

        // le joueur de l'autre équipe n'est jamais touché
        let other_team_after = repo.find_by_id(&PlayerId("other_team_injured".into())).await.unwrap().unwrap();
        assert_eq!(other_team_after.version, 2); // seulement sa blessure initiale, pas de MatchConcluded
    }

    #[sqlx::test]
    async fn match_concluded_increments_matches_played_for_every_player_regardless_of_status(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);

        let injured = seed_player(&repo, "injured", "t1").await;
        let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&injured.id, &injured.team_id, &injury_event, 2).await.unwrap();

        seed_player(&repo, "healthy", "t1").await;

        handle_team_match_concluded(&repo, "t1", concluded_context(), 2, 1).await;

        let injured_after = repo.find_by_id(&PlayerId("injured".into())).await.unwrap().unwrap();
        assert_eq!(injured_after.matches_played.0, 1);
        assert_eq!(injured_after.participation_status, PlayerParticipationStatus::Available);

        let healthy_after = repo.find_by_id(&PlayerId("healthy".into())).await.unwrap().unwrap();
        assert_eq!(healthy_after.matches_played.0, 1);
        assert_eq!(healthy_after.version, 2); // MatchConcluded seulement, pas de restauration inutile
    }
}
