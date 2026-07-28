use crate::app::players::domain::match_impact::{InjuryType, MatchContext, MatchReportId, RoundId, SppEarned, StatKind};
use crate::app::players::domain::player::{Player, PlayerId, TeamId};
use crate::app::players::io::app_events::team_match_concluded_listener::handle_team_match_concluded;
use crate::app::players::ports::IPlayerRepository;
use crate::app::players::ports::ISkillCatalogPort;
use crate::app::shared_kernel::app_events::player_match_impact_app_events::{
    InjuryTypePayload, PlayerMatchContextPayload, PlayerMatchImpactAppEvent,
};
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

pub fn init(
    app_event_bus: &EventBus,
    player_repo: Arc<dyn IPlayerRepository>,
    skill_catalog: Arc<dyn ISkillCatalogPort>,
) {
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
                    handle_event(app_event, player_repo.as_ref(), skill_catalog.as_ref()).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("player_match_impact_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(
    app_event: PlayerMatchImpactAppEvent,
    player_repo: &dyn IPlayerRepository,
    catalog: &dyn ISkillCatalogPort,
) {
    // Traité dans cette même tâche séquentielle (et non par un listener à part) pour
    // garantir que TeamMatchConcluded n'est jamais appliqué avant les events d'action
    // du même match — sinon les deux tâches se disputent la même version du joueur.
    if let PlayerMatchImpactAppEvent::TeamMatchConcluded { .. } = &app_event {
        return dispatch_team_match_concluded(app_event, player_repo).await;
    }

    // Comme `TeamMatchConcluded`, cet événement porte sur toute une équipe et
    // non sur un joueur : il est traité à part, dans la même tâche séquentielle.
    if let PlayerMatchImpactAppEvent::TeamMatchImpactReverted { team_id, match_report_id } =
        &app_event
    {
        return revert_team_match_impact(player_repo, team_id, match_report_id).await;
    }

    let (context_payload, player) = match &app_event {
        PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(c)
        | PlayerMatchImpactAppEvent::PlayerPerformedPass(c)
        | PlayerMatchImpactAppEvent::PlayerPerformedInterception(c)
        | PlayerMatchImpactAppEvent::PlayerPerformedCasualty(c)
        | PlayerMatchImpactAppEvent::PlayerPerformedMvp(c)
        | PlayerMatchImpactAppEvent::PlayerPerformedFoul(c) => (c.clone(), load_player(player_repo, &c.player_id).await),
        PlayerMatchImpactAppEvent::PlayerInjured { context, .. } => {
            (context.clone(), load_player(player_repo, &context.player_id).await)
        }
        PlayerMatchImpactAppEvent::TeamMatchConcluded { .. }
        | PlayerMatchImpactAppEvent::TeamMatchImpactReverted { .. } => {
            unreachable!("traités plus haut")
        }
    };

    let Some(player) = player else {
        tracing::warn!(
            "player_match_impact_listener: joueur {} introuvable",
            context_payload.player_id
        );
        return;
    };

    let context = to_match_context(&context_payload);

    let event = match app_event {
        PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(_) => {
            player.record_touchdown(context, spp_earned(catalog.touchdown_spp()))
        }
        PlayerMatchImpactAppEvent::PlayerPerformedPass(_) => {
            player.record_pass(context, spp_earned(catalog.pass_spp()))
        }
        PlayerMatchImpactAppEvent::PlayerPerformedInterception(_) => {
            player.record_interception(context, spp_earned(catalog.interception_spp()))
        }
        PlayerMatchImpactAppEvent::PlayerPerformedCasualty(_) => {
            player.record_casualty(context, spp_earned(catalog.casualty_spp()))
        }
        PlayerMatchImpactAppEvent::PlayerPerformedMvp(_) => {
            player.record_mvp(context, spp_earned(catalog.mvp_spp()))
        }
        PlayerMatchImpactAppEvent::PlayerPerformedFoul(_) => player.record_foul(context),
        PlayerMatchImpactAppEvent::PlayerInjured { injury_type, .. } => {
            player.record_injury(context, to_injury_type(&injury_type))
        }
        PlayerMatchImpactAppEvent::TeamMatchConcluded { .. }
        | PlayerMatchImpactAppEvent::TeamMatchImpactReverted { .. } => unreachable!(),
    };

    let next_version = player.version + 1;
    if let Err(e) = player_repo.append(&player.id, &player.team_id, &event, next_version).await {
        tracing::error!(
            "player_match_impact_listener: append {}: {e}",
            context_payload.player_id
        );
    }
}

async fn dispatch_team_match_concluded(
    app_event: PlayerMatchImpactAppEvent,
    player_repo: &dyn IPlayerRepository,
) {
    let PlayerMatchImpactAppEvent::TeamMatchConcluded {
        team_id, match_report_id, round_id, round_label,
        opponent_team_id, opponent_team_name, team_score, opponent_score,
    } = app_event
    else {
        return;
    };
    let context = MatchContext {
        match_report_id: MatchReportId(match_report_id),
        round_id:        RoundId(round_id),
        round_label,
        opponent_team_id:   TeamId(opponent_team_id),
        opponent_team_name,
    };
    handle_team_match_concluded(player_repo, &team_id, context, team_score, opponent_score).await;
}

/// Défait l'impact d'un match sur tout l'effectif d'une équipe.
///
/// Itère sans se soucier de qui a joué : le domaine renvoie `None` pour les
/// joueurs dont le dernier match n'est pas celui-ci, ce qui vaut aussi
/// idempotence si la compensation est rejouée.
async fn revert_team_match_impact(
    player_repo:     &dyn IPlayerRepository,
    team_id:         &str,
    match_report_id: &str,
) {
    let Some(players) = load_roster(player_repo, team_id).await else {
        return;
    };
    let target = MatchReportId(match_report_id.to_string());
    for player in &players {
        revert_one_player(player_repo, player, &target).await;
    }
}

async fn revert_one_player(
    player_repo: &dyn IPlayerRepository,
    player:      &Player,
    target:      &MatchReportId,
) {
    // `None` : ce joueur n'a rien à défaire pour ce match.
    let Some(event) = player.revert_match_impact(target) else {
        return;
    };
    if let Err(e) = player_repo
        .append(&player.id, &player.team_id, &event, player.version + 1)
        .await
    {
        tracing::error!(
            "player_match_impact_listener: append MatchImpactReverted {}: {e}",
            player.id.0
        );
    }
}

async fn load_roster(player_repo: &dyn IPlayerRepository, team_id: &str) -> Option<Vec<Player>> {
    match player_repo.find_by_team_id(&TeamId(team_id.to_string())).await {
        Ok(players) => Some(players),
        Err(e) => {
            tracing::error!("player_match_impact_listener: find_by_team_id {team_id}: {e}");
            None
        }
    }
}

async fn load_player(player_repo: &dyn IPlayerRepository, player_id: &str) -> Option<Player> {
    player_repo.find_by_id(&PlayerId(player_id.to_string())).await.ok().flatten()
}

fn spp_earned(amount: u8) -> SppEarned {
    // Le barème `references` garantit toujours >= 1 (BR4) — panique volontaire sinon.
    SppEarned::try_new(amount as u32).expect("le barème SPP doit toujours être >= 1")
}

fn to_match_context(c: &PlayerMatchContextPayload) -> MatchContext {
    MatchContext {
        match_report_id:    MatchReportId(c.match_report_id.clone()),
        round_id:           RoundId(c.round_id.clone()),
        round_label:        c.round_label.clone(),
        opponent_team_id:   TeamId(c.opponent_team_id.clone()),
        opponent_team_name: c.opponent_team_name.clone(),
    }
}

fn to_injury_type(payload: &InjuryTypePayload) -> InjuryType {
    match payload {
        InjuryTypePayload::Commotion => InjuryType::Commotion,
        InjuryTypePayload::Amoche => InjuryType::Amoche,
        InjuryTypePayload::BlessureSerieuse => InjuryType::BlessureSerieuse,
        InjuryTypePayload::Sequel { stat } => InjuryType::Sequel { stat: to_stat_kind(stat) },
        InjuryTypePayload::Mort => InjuryType::Mort,
    }
}

fn to_stat_kind(stat: &str) -> StatKind {
    match stat {
        "Ma" => StatKind::Ma,
        "St" => StatKind::St,
        "Ag" => StatKind::Ag,
        "Pa" => StatKind::Pa,
        "Av" => StatKind::Av,
        other => {
            tracing::warn!("player_match_impact_listener: stat inconnue '{other}', défaut Ag");
            StatKind::Ag
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::PlayerParticipationStatus;
    use crate::app::players::domain::player::{Spp, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId};
    use crate::app::players::io::repository::player_repository::PgPlayerRepository;
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;
    use sqlx::PgPool;
    use std::sync::Arc;

    fn test_catalog() -> SkillCatalogAdapter {
        SkillCatalogAdapter::new(Arc::new(InMemoryReferenceRepository::load_for_tests()))
    }

    fn sample_context_payload(player_id: &str) -> PlayerMatchContextPayload {
        PlayerMatchContextPayload {
            match_report_id:    "mr1".into(),
            round_id:           "r1".into(),
            round_label:        "Journée 5".into(),
            opponent_team_id:   "opponent".into(),
            opponent_team_name: "Bone Crushers".into(),
            player_id:          player_id.to_string(),
        }
    }

    async fn seed_player(repo: &PgPlayerRepository, player_id: &str, team_id: &str) {
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

    #[sqlx::test]
    async fn touchdown_event_credits_spp_on_existing_player(pool: PgPool) {
        let player_repo = PgPlayerRepository::new(pool);
        let catalog = test_catalog();
        seed_player(&player_repo, "p1", "t1").await;

        handle_event(
            PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(sample_context_payload("p1")),
            &player_repo,
            &catalog,
        )
        .await;

        let player = player_repo.find_by_id(&PlayerId("p1".into())).await.unwrap().unwrap();
        assert_eq!(player.spp.0, 3);
        assert_eq!(player.career_touchdowns.0, 1);
        assert_eq!(player.version, 2);
    }

    #[sqlx::test]
    async fn injury_event_updates_participation_status(pool: PgPool) {
        let player_repo = PgPlayerRepository::new(pool);
        let catalog = test_catalog();
        seed_player(&player_repo, "p2", "t1").await;

        handle_event(
            PlayerMatchImpactAppEvent::PlayerInjured {
                context:     sample_context_payload("p2"),
                injury_type: InjuryTypePayload::BlessureSerieuse,
            },
            &player_repo,
            &catalog,
        )
        .await;

        let player = player_repo.find_by_id(&PlayerId("p2".into())).await.unwrap().unwrap();
        assert_eq!(player.participation_status, PlayerParticipationStatus::MissingNextGame);
        assert_eq!(player.career_persistent_injuries.0, 1);
    }

    #[sqlx::test]
    async fn unknown_player_is_ignored_without_panicking(pool: PgPool) {
        let player_repo = PgPlayerRepository::new(pool);
        let catalog = test_catalog();

        handle_event(
            PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(sample_context_payload("ghost")),
            &player_repo,
            &catalog,
        )
        .await;
    }
}
