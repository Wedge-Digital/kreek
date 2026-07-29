use crate::app::players::domain::match_impact::{
    MatchContext, MatchReportId, PlayerParticipationStatus,
};
use crate::app::players::domain::player::{Player, TeamId};
use crate::app::players::ports::IPlayerRepository;

/// Pour chaque joueur d'une équipe dont le rapport de match vient de se
/// conclure : enregistre toujours `MatchConcluded` (compteur de matchs joués +
/// ancre d'historique), et en plus lève `MissingNextGame` → `Available` (BR12)
/// pour ceux qui l'étaient **avant** ce match — peu importe que le joueur ait
/// vraiment joué ou non.
///
/// La restriction « avant ce match » est essentielle : le publisher émet les
/// events d'action et `TeamMatchConcluded` avec le même `match_report_id`, dans
/// la même tâche séquentielle. Restaurer sans distinction annulerait la
/// blessure subie pendant ce match-là, au moment même où elle vient d'être
/// enregistrée (carte 225).
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
    let players = match player_repo
        .find_by_team_id(&TeamId(team_id.to_string()))
        .await
    {
        Ok(players) => players,
        Err(e) => {
            tracing::error!("team_match_concluded_listener: find_by_team_id {team_id}: {e}");
            return;
        }
    };

    for player in &players {
        let concluded = player.record_match_concluded(context.clone(), team_score, opponent_score);
        let next_version = player.version + 1;
        if let Err(e) = player_repo
            .append(&player.id, &player.team_id, &concluded, next_version)
            .await
        {
            tracing::error!(
                "team_match_concluded_listener: append MatchConcluded {}: {e}",
                player.id.0
            );
            continue;
        }

        if is_restorable(player, &context.match_report_id) {
            let restored = player.restore_availability(context.match_report_id.clone());
            if let Err(e) = player_repo
                .append(&player.id, &player.team_id, &restored, next_version + 1)
                .await
            {
                tracing::error!(
                    "team_match_concluded_listener: append PlayerAvailabilityRestored {}: {e}",
                    player.id.0
                );
            }
        }
    }
}

/// Le joueur était-il absent **avant** ce match ?
///
/// `injuries` porte le `match_report_id` de chaque blessure : il suffit de
/// regarder si celle qui l'a rendu indisponible vient de ce match-ci. Aucune
/// donnée supplémentaire à stocker.
///
/// Un joueur mort ou retiré n'est pas concerné — seul `MissingNextGame` se
/// restaure.
fn is_restorable(player: &Player, current_match: &MatchReportId) -> bool {
    if player.participation_status != PlayerParticipationStatus::MissingNextGame {
        return false;
    }
    !player
        .injuries
        .iter()
        .any(|i| &i.context.match_report_id == current_match)
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
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use sqlx::PgPool;

    fn sample_context() -> MatchContext {
        MatchContext {
            match_report_id: MatchReportId("mr1".into()),
            round_id: MatchImpactRoundId("r1".into()),
            round_label: "Journée 5".into(),
            opponent_team_id: TeamId("opponent".into()),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    async fn seed_player(
        repo: &PgPlayerRepository,
        player_id: &str,
        team_id: &str,
    ) -> crate::app::players::domain::player::Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId(player_id.to_string()),
            team_id: TeamId(team_id.to_string()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100_000),
        };
        repo.append(
            &PlayerId(player_id.into()),
            &TeamId(team_id.into()),
            &created,
            1,
        )
        .await
        .unwrap();
        crate::app::players::domain::player::Player::from_events(&[created]).unwrap()
    }

    fn concluded_context() -> MatchContext {
        MatchContext {
            match_report_id: MatchReportId("mr2".into()),
            round_id: MatchImpactRoundId("r2".into()),
            round_label: "Journée 6".into(),
            opponent_team_id: TeamId("opponent2".into()),
            opponent_team_name: "Green Machine".into(),
        }
    }

    #[sqlx::test]
    async fn restores_only_missing_next_game_players_of_the_team(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);

        let injured = seed_player(&repo, "injured", "t1").await;
        let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&injured.id, &injured.team_id, &injury_event, 2)
            .await
            .unwrap();

        seed_player(&repo, "healthy", "t1").await;
        let other_team_injured = seed_player(&repo, "other_team_injured", "t2").await;
        let other_team_injury_event =
            other_team_injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(
            &other_team_injured.id,
            &other_team_injured.team_id,
            &other_team_injury_event,
            2,
        )
        .await
        .unwrap();

        handle_team_match_concluded(&repo, "t1", concluded_context(), 2, 1).await;

        let injured_after = repo
            .find_by_id(&PlayerId("injured".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            injured_after.participation_status,
            PlayerParticipationStatus::Available
        );

        // le joueur de l'autre équipe n'est jamais touché : ni restauré, ni MatchConcluded ajouté
        let other_team_after = repo
            .find_by_id(&PlayerId("other_team_injured".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            other_team_after.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );
        assert_eq!(other_team_after.version, 2); // seulement sa blessure initiale, pas de MatchConcluded
    }

    #[sqlx::test]
    async fn match_concluded_increments_matches_played_for_every_player_regardless_of_status(
        pool: PgPool,
    ) {
        let repo = PgPlayerRepository::new(pool);

        let injured = seed_player(&repo, "injured", "t1").await;
        let injury_event = injured.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&injured.id, &injured.team_id, &injury_event, 2)
            .await
            .unwrap();

        seed_player(&repo, "healthy", "t1").await;

        handle_team_match_concluded(&repo, "t1", concluded_context(), 2, 1).await;

        let injured_after = repo
            .find_by_id(&PlayerId("injured".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(injured_after.matches_played.0, 1);
        assert_eq!(
            injured_after.participation_status,
            PlayerParticipationStatus::Available
        );

        let healthy_after = repo
            .find_by_id(&PlayerId("healthy".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(healthy_after.matches_played.0, 1);
        assert_eq!(healthy_after.version, 2); // MatchConcluded seulement, pas de restauration inutile
    }
    /// Carte 225 : la blessure subie **pendant** le match qui se conclut ne doit
    /// pas être annulée par cette conclusion. Le publisher émet les deux events
    /// avec le même `match_report_id`, dos à dos.
    #[sqlx::test]
    async fn une_blessure_subie_pendant_ce_match_n_est_pas_restauree(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);
        let joueur = seed_player(&repo, "blesse", "t1").await;
        // blessure portant le MÊME match que la conclusion qui suit
        let blessure = joueur.record_injury(sample_context(), InjuryType::BlessureSerieuse);
        repo.append(&joueur.id, &joueur.team_id, &blessure, 2)
            .await
            .unwrap();

        handle_team_match_concluded(&repo, "t1", sample_context(), 1, 0).await;

        let apres = repo
            .find_by_id(&PlayerId("blesse".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            apres.participation_status,
            PlayerParticipationStatus::MissingNextGame,
            "le joueur doit rester absent au prochain match"
        );
    }

    /// Le pendant du test ci-dessus : une blessure d'un match **antérieur** se
    /// restaure bien, c'est l'objet même de BR12.
    #[sqlx::test]
    async fn une_blessure_d_un_match_anterieur_est_restauree(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);
        let joueur = seed_player(&repo, "blesse", "t1").await;
        let blessure = joueur.record_injury(sample_context(), InjuryType::BlessureSerieuse);
        repo.append(&joueur.id, &joueur.team_id, &blessure, 2)
            .await
            .unwrap();

        // conclusion d'un match différent (mr2), soit le match suivant
        handle_team_match_concluded(&repo, "t1", concluded_context(), 1, 0).await;

        let apres = repo
            .find_by_id(&PlayerId("blesse".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            apres.participation_status,
            PlayerParticipationStatus::Available
        );
    }

    /// Un joueur blessé au match N puis de nouveau au match N+1 doit rester
    /// absent : la conclusion de N+1 restaure la blessure de N, mais celle de
    /// N+1 vient d'être posée.
    #[sqlx::test]
    async fn une_nouvelle_blessure_prime_sur_la_restauration_de_l_ancienne(pool: PgPool) {
        let repo = PgPlayerRepository::new(pool);
        let joueur = seed_player(&repo, "blesse", "t1").await;
        let ancienne = joueur.record_injury(sample_context(), InjuryType::Amoche);
        repo.append(&joueur.id, &joueur.team_id, &ancienne, 2)
            .await
            .unwrap();

        let joueur = repo
            .find_by_id(&PlayerId("blesse".into()))
            .await
            .unwrap()
            .unwrap();
        let nouvelle = joueur.record_injury(concluded_context(), InjuryType::Amoche);
        repo.append(&joueur.id, &joueur.team_id, &nouvelle, 3)
            .await
            .unwrap();

        handle_team_match_concluded(&repo, "t1", concluded_context(), 1, 0).await;

        let apres = repo
            .find_by_id(&PlayerId("blesse".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            apres.participation_status,
            PlayerParticipationStatus::MissingNextGame
        );
    }
}
