use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    ActionPlayer, InjuryType, MatchAction, MatchActionType, TempPlayer, TempPlayerKind,
};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload, PlayerRefPayload, TempPlayerPayload,
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
