use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    ActionPlayer, InjuryType, MatchAction, MatchActionType, SequelStat, TempPlayer, TempPlayerKind,
};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload, PlayerRefPayload, TempPlayerPayload,
};
use crate::app::shared_kernel::app_events::player_match_impact_app_events::{
    InjuryTypePayload, PlayerMatchContextPayload, PlayerMatchImpactAppEvent,
};
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Souscrit au bus interne du BC match_report, convertit les domain events pertinents
/// en app events, et les republie sur l'app event bus. Même pattern que
/// `competitions_app_event_publisher`.
pub fn match_report_app_event_publisher(
    event_bus: &EventBus,
    app_event_bus: EventBus,
    repo: Arc<dyn IMatchReportRepository>,
    competition_data: Arc<dyn ICompetitionDataPort>,
    team_data: Arc<dyn ITeamDataPort>,
) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<MatchReportDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    if !matches!(event, MatchReportDomainEvent::MatchReportPublished { .. }) {
                        continue;
                    }
                    let match_report_id = envelope.emitter.clone();
                    match repo.find_by_id(&match_report_id).await {
                        Ok(Some(MatchReportState::Published(p))) => {
                            let payload = build_published_payload(&p);
                            let _ = app_event_bus
                                .send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());

                            publish_player_impact_events(
                                &p,
                                &app_event_bus,
                                competition_data.as_ref(),
                                team_data.as_ref(),
                            )
                            .await;
                        }
                        Ok(_) => {
                            tracing::warn!(
                                "match_report_app_event_publisher: {match_report_id} pas en état Published après MatchReportPublished"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "match_report_app_event_publisher: find_by_id {match_report_id}: {e}"
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

// ── Player report events (impact sur les joueurs) ────────────────────────────────

async fn publish_player_impact_events(
    p: &MatchReportPublished,
    app_event_bus: &EventBus,
    competition_data: &dyn ICompetitionDataPort,
    team_data: &dyn ITeamDataPort,
) {
    let round_label = competition_data
        .find_round_context(&p.season_id.to_string(), &p.round_id.to_string())
        .await
        .map(|c| c.round_name)
        .unwrap_or_default();
    let home_team_name = team_data
        .find_team_info(&p.home_team_id.to_string())
        .await
        .map(|t| t.team_name)
        .unwrap_or_default();
    let away_team_name = team_data
        .find_team_info(&p.away_team_id.to_string())
        .await
        .map(|t| t.team_name)
        .unwrap_or_default();

    let home_ctx_base = ContextBase {
        match_report_id:    p.id.to_string(),
        round_id:           p.round_id.to_string(),
        round_label:        round_label.clone(),
        opponent_team_id:   p.away_team_id.to_string(),
        opponent_team_name: away_team_name,
    };
    let away_ctx_base = ContextBase {
        match_report_id:    p.id.to_string(),
        round_id:           p.round_id.to_string(),
        round_label,
        opponent_team_id:   p.home_team_id.to_string(),
        opponent_team_name: home_team_name,
    };

    for event in build_player_impact_events(&p.home_actions, &home_ctx_base)
        .into_iter()
        .chain(build_player_impact_events(&p.away_actions, &away_ctx_base))
    {
        let _ = app_event_bus.send(event.to_enveloppe());
    }

    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::TeamMatchConcluded {
            team_id:         p.home_team_id.to_string(),
            match_report_id: p.id.to_string(),
        }
        .to_enveloppe(),
    );
    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::TeamMatchConcluded {
            team_id:         p.away_team_id.to_string(),
            match_report_id: p.id.to_string(),
        }
        .to_enveloppe(),
    );
}

/// Contexte commun à un camp (home ou away) — tout sauf `player_id`, résolu une
/// seule fois par publication (pas par action).
struct ContextBase {
    match_report_id:    String,
    round_id:            String,
    round_label:         String,
    opponent_team_id:    String,
    opponent_team_name:  String,
}

impl ContextBase {
    fn for_player(&self, player_id: &str) -> PlayerMatchContextPayload {
        PlayerMatchContextPayload {
            match_report_id:    self.match_report_id.clone(),
            round_id:           self.round_id.clone(),
            round_label:        self.round_label.clone(),
            opponent_team_id:   self.opponent_team_id.clone(),
            opponent_team_name: self.opponent_team_name.clone(),
            player_id:          player_id.to_string(),
        }
    }
}

fn build_player_impact_events(
    actions: &[MatchAction],
    ctx_base: &ContextBase,
) -> Vec<PlayerMatchImpactAppEvent> {
    actions
        .iter()
        .filter_map(|a| {
            let ActionPlayer::Regular(player_id) = &a.player else {
                return None; // BR1 — stars/mercenaires/journaliers exclus
            };
            let context = ctx_base.for_player(&player_id.to_string());
            map_action_to_impact_event(&a.action, context)
        })
        .collect()
}

fn map_action_to_impact_event(
    action: &MatchActionType,
    context: PlayerMatchContextPayload,
) -> Option<PlayerMatchImpactAppEvent> {
    Some(match action {
        MatchActionType::Touchdown => PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(context),
        // BR2 — Passe et Lancer sont la même notion domaine
        MatchActionType::Passe | MatchActionType::Lancer => {
            PlayerMatchImpactAppEvent::PlayerPerformedPass(context)
        }
        MatchActionType::Interception => PlayerMatchImpactAppEvent::PlayerPerformedInterception(context),
        MatchActionType::Sortie => PlayerMatchImpactAppEvent::PlayerPerformedCasualty(context),
        MatchActionType::Mvp => PlayerMatchImpactAppEvent::PlayerPerformedMvp(context),
        MatchActionType::Agression => PlayerMatchImpactAppEvent::PlayerPerformedFoul(context),
        MatchActionType::Blesse { injury } => PlayerMatchImpactAppEvent::PlayerInjured {
            context,
            injury_type: map_injury_type_payload(injury),
        },
    })
}

fn map_injury_type_payload(injury: &InjuryType) -> InjuryTypePayload {
    match injury {
        InjuryType::Commotion => InjuryTypePayload::Commotion,
        InjuryType::Amoche => InjuryTypePayload::Amoche,
        InjuryType::BlessureSerieuse => InjuryTypePayload::BlessureSerieuse,
        InjuryType::Sequel { stat } => InjuryTypePayload::Sequel { stat: map_sequel_stat(*stat).to_string() },
        InjuryType::Mort => InjuryTypePayload::Mort,
    }
}

fn map_sequel_stat(stat: SequelStat) -> &'static str {
    match stat {
        SequelStat::MinusMa => "Ma",
        SequelStat::MinusSt => "St",
        SequelStat::MinusAg => "Ag",
        SequelStat::MinusPa => "Pa",
        SequelStat::MinusAv => "Av",
    }
}

fn build_published_payload(p: &MatchReportPublished) -> MatchReportPublishedPayload {
    MatchReportPublishedPayload {
        match_report_id: p.id.to_string(),
        space_id: p.space_id.to_string(),
        competition_id: p.competition_id.to_string(),
        season_id: p.season_id.to_string(),
        round_id: p.round_id.to_string(),
        pairing_id: p.pairing_id.clone(),
        published_at: p.published_at,
        home_team_id: p.home_team_id.to_string(),
        away_team_id: p.away_team_id.to_string(),
        home_score: count_touchdowns(&p.home_actions),
        away_score: count_touchdowns(&p.away_actions),
        home_gain_kpo: p.home_gain.into_inner(),
        away_gain_kpo: p.away_gain.into_inner(),
        home_fan_mod: p.home_fan_mod.into_inner(),
        away_fan_mod: p.away_fan_mod.into_inner(),
        home_actions: build_action_payloads(&p.home_actions, &p.home_temp_players),
        away_actions: build_action_payloads(&p.away_actions, &p.away_temp_players),
        home_temp_players: p.home_temp_players.iter().map(build_temp_player_payload).collect(),
        away_temp_players: p.away_temp_players.iter().map(build_temp_player_payload).collect(),
    }
}

fn count_touchdowns(actions: &[MatchAction]) -> u8 {
    actions.iter().filter(|a| matches!(a.action, MatchActionType::Touchdown)).count() as u8
}

fn build_action_payloads(
    actions: &[MatchAction],
    temp_players: &[TempPlayer],
) -> Vec<MatchActionPublishedPayload> {
    actions
        .iter()
        .map(|a| MatchActionPublishedPayload {
            turn: a.turn.value(),
            player: build_player_ref(&a.player, temp_players),
            action: build_action_type(&a.action),
        })
        .collect()
}

fn build_player_ref(player: &ActionPlayer, temp_players: &[TempPlayer]) -> PlayerRefPayload {
    match player {
        ActionPlayer::Regular(player_id) => PlayerRefPayload::Regular { player_id: player_id.to_string() },
        ActionPlayer::Temp(temp_id) => temp_players
            .iter()
            .find(|t| &t.id == temp_id)
            .map(|t| match &t.kind {
                TempPlayerKind::StarPlayer { ref_uid, .. } => PlayerRefPayload::Star {
                    ref_uid: ref_uid.clone(),
                    display_name: t.display_name.clone().unwrap_or_default(),
                },
                TempPlayerKind::Mercenary { .. } => PlayerRefPayload::Mercenary,
                TempPlayerKind::Journalier { .. } => PlayerRefPayload::Journalier,
            })
            .unwrap_or(PlayerRefPayload::Journalier),
    }
}

fn build_action_type(action: &MatchActionType) -> ActionTypePayload {
    match action {
        MatchActionType::Touchdown => ActionTypePayload::Touchdown,
        MatchActionType::Passe => ActionTypePayload::Passe,
        MatchActionType::Interception => ActionTypePayload::Interception,
        MatchActionType::Agression => ActionTypePayload::Agression,
        MatchActionType::Lancer => ActionTypePayload::Lancer,
        MatchActionType::Sortie => ActionTypePayload::Sortie,
        MatchActionType::Mvp => ActionTypePayload::Mvp,
        MatchActionType::Blesse { injury } => {
            ActionTypePayload::Blesse { injury: injury_label(injury) }
        }
    }
}

fn injury_label(injury: &InjuryType) -> String {
    match injury {
        InjuryType::Commotion => "Commotion".to_string(),
        InjuryType::Amoche => "Amoche".to_string(),
        InjuryType::BlessureSerieuse => "BlessureSerieuse".to_string(),
        InjuryType::Sequel { .. } => "Sequel".to_string(),
        InjuryType::Mort => "Mort".to_string(),
    }
}

fn build_temp_player_payload(t: &TempPlayer) -> TempPlayerPayload {
    let kind = match &t.kind {
        TempPlayerKind::StarPlayer { .. } => "StarPlayer",
        TempPlayerKind::Mercenary { .. } => "Mercenary",
        TempPlayerKind::Journalier { .. } => "Journalier",
    };
    TempPlayerPayload {
        id: t.id.0.clone(),
        kind: kind.to_string(),
        display_name: t.display_name.clone(),
    }
}

#[cfg(test)]
mod player_impact_tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{ActionId, TempPlayerId, TurnNumber};
    use crate::app::shared_kernel::common_types::PlayerId as SharedPlayerId;

    fn ctx_base() -> ContextBase {
        ContextBase {
            match_report_id:    "mr1".into(),
            round_id:           "r1".into(),
            round_label:        "Journée 5".into(),
            opponent_team_id:   "opponent".into(),
            opponent_team_name: "Bone Crushers".into(),
        }
    }

    fn regular_action(action: MatchActionType) -> MatchAction {
        MatchAction {
            id: ActionId("a1".into()),
            turn: TurnNumber::try_new(1).unwrap(),
            player: ActionPlayer::Regular(SharedPlayerId::new()),
            action,
            player_display_name: "Tyrandel".into(),
            player_position: "Frappeur".into(),
        }
    }

    fn temp_action(action: MatchActionType) -> MatchAction {
        MatchAction {
            id: ActionId("a2".into()),
            turn: TurnNumber::try_new(1).unwrap(),
            player: ActionPlayer::Temp(TempPlayerId("star1".into())),
            action,
            player_display_name: "Griff Oberwald".into(),
            player_position: "Star".into(),
        }
    }

    #[test]
    fn regular_touchdown_maps_to_player_performed_touchdown() {
        let events = build_player_impact_events(&[regular_action(MatchActionType::Touchdown)], &ctx_base());
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(_)));
    }

    #[test]
    fn passe_and_lancer_both_map_to_player_performed_pass() {
        let events = build_player_impact_events(
            &[regular_action(MatchActionType::Passe), regular_action(MatchActionType::Lancer)],
            &ctx_base(),
        );
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| matches!(e, PlayerMatchImpactAppEvent::PlayerPerformedPass(_))));
    }

    #[test]
    fn agression_maps_to_foul_without_spp_bearing_variant() {
        let events = build_player_impact_events(&[regular_action(MatchActionType::Agression)], &ctx_base());
        assert!(matches!(events[0], PlayerMatchImpactAppEvent::PlayerPerformedFoul(_)));
    }

    #[test]
    fn blesse_sequel_maps_to_player_injured_with_structured_stat() {
        let events = build_player_impact_events(
            &[regular_action(MatchActionType::Blesse { injury: InjuryType::Sequel { stat: SequelStat::MinusAg } })],
            &ctx_base(),
        );
        match &events[0] {
            PlayerMatchImpactAppEvent::PlayerInjured { injury_type, .. } => {
                assert!(matches!(injury_type, InjuryTypePayload::Sequel { stat } if stat == "Ag"));
            }
            other => panic!("expected PlayerInjured, got {other:?}"),
        }
    }

    #[test]
    fn temp_player_actions_are_excluded() {
        let events = build_player_impact_events(&[temp_action(MatchActionType::Touchdown)], &ctx_base());
        assert!(events.is_empty());
    }
}
