use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{
    ActionPlayer, InducementSpending, InjuryType, MatchAction, MatchActionType, SequelStat,
    TempPlayer, TempPlayerKind,
};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload, MatchReportUnpublishedPayload, PlayerRefPayload,
    TempPlayerPayload,
};
use crate::app::shared_kernel::app_events::player_match_impact_app_events::{
    InjuryTypePayload, PlayerMatchContextPayload, PlayerMatchImpactAppEvent,
};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
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
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    handle_envelope(
                        envelope,
                        &app_event_bus,
                        repo.as_ref(),
                        competition_data.as_ref(),
                        team_data.as_ref(),
                    )
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Aiguille sur les seuls événements qui franchissent la frontière du BC.
async fn handle_envelope(
    envelope: crate::common::event_envelope::EventEnvelope,
    app_event_bus: &EventBus,
    repo: &dyn IMatchReportRepository,
    competition_data: &dyn ICompetitionDataPort,
    team_data: &dyn ITeamDataPort,
) {
    let Ok(event) = serde_json::from_value::<MatchReportDomainEvent>(envelope.payload.clone())
    else {
        return;
    };
    let match_report_id = envelope.emitter;
    match event {
        MatchReportDomainEvent::MatchReportPublished { .. } => {
            handle_published(
                &match_report_id,
                app_event_bus,
                repo,
                competition_data,
                team_data,
            )
            .await
        }
        MatchReportDomainEvent::MatchReportUnpublished { .. } => {
            handle_unpublished(&match_report_id, app_event_bus, repo).await
        }
        MatchReportDomainEvent::MatchReportCancelled {
            home_team_id,
            away_team_id,
            ..
        } => handle_cancelled(&match_report_id, home_team_id, away_team_id, app_event_bus),
        _ => {}
    }
}

/// Contrairement aux deux autres, cet app event se construit depuis
/// l'événement lui-même : relire l'agrégat ne donnerait qu'un état `Cancelled`,
/// qui ne retient plus les équipes.
///
/// Les annulations persistées avant l'ajout des ids d'équipes ne portent aucun
/// verrou à défaire — elles venaient toutes d'un brouillon — donc rien à
/// publier.
fn handle_cancelled(
    match_report_id: &str,
    home_team_id: Option<TeamId>,
    away_team_id: Option<TeamId>,
    app_event_bus: &EventBus,
) {
    let (Some(home), Some(away)) = (home_team_id, away_team_id) else {
        tracing::debug!(
            "app_event_publisher: MatchReportCancelled sans équipes ({match_report_id}), \
             ancien format — aucun app event publié"
        );
        return;
    };

    let _ = app_event_bus.send(
        MatchReportAppEvent::MatchReportCancelled {
            event_id: EventId::new(),
            match_report_id: match_report_id.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
        }
        .to_enveloppe(),
    );
}

async fn handle_published(
    match_report_id: &str,
    app_event_bus: &EventBus,
    repo: &dyn IMatchReportRepository,
    competition_data: &dyn ICompetitionDataPort,
    team_data: &dyn ITeamDataPort,
) {
    match repo.find_by_id(match_report_id).await {
        Ok(Some(MatchReportState::Published(p))) => {
            let payload = build_published_payload(&p);
            let _ = app_event_bus
                .send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());
            publish_player_impact_events(&p, app_event_bus, competition_data, team_data).await;
        }
        Ok(_) => log_unexpected_state(match_report_id, "Published"),
        Err(e) => log_reread_error(match_report_id, e),
    }
}

/// La relecture attend ici `ReadyToPublish` et non `Published` : c'est l'état
/// dans lequel la dépublication vient de laisser le rapport. Exiger `Published`
/// ferait échouer toutes les compensations en silence, avec un simple `warn!`.
async fn handle_unpublished(
    match_report_id: &str,
    app_event_bus: &EventBus,
    repo: &dyn IMatchReportRepository,
) {
    match repo.find_by_id(match_report_id).await {
        Ok(Some(MatchReportState::ReadyToPublish(rtp))) => {
            for event in build_unpublished_events(&rtp) {
                let _ = app_event_bus.send(event);
            }
        }
        Ok(_) => log_unexpected_state(match_report_id, "ReadyToPublish"),
        Err(e) => log_reread_error(match_report_id, e),
    }
}

/// Un état inattendu signifie qu'aucun app event ne partira : sans cette trace,
/// la compensation échouerait sans laisser d'indice.
fn log_unexpected_state(match_report_id: &str, expected: &str) {
    tracing::warn!(
        "match_report_app_event_publisher: {match_report_id} n'est pas en état {expected}"
    );
}

fn log_reread_error(match_report_id: &str, e: impl std::fmt::Display) {
    tracing::error!("match_report_app_event_publisher: find_by_id {match_report_id}: {e}");
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
        match_report_id: p.id.to_string(),
        round_id: p.round_id.to_string(),
        round_label: round_label.clone(),
        opponent_team_id: p.away_team_id.to_string(),
        opponent_team_name: away_team_name,
    };
    let away_ctx_base = ContextBase {
        match_report_id: p.id.to_string(),
        round_id: p.round_id.to_string(),
        round_label,
        opponent_team_id: p.home_team_id.to_string(),
        opponent_team_name: home_team_name,
    };

    for event in build_player_impact_events(&p.home_actions, &home_ctx_base)
        .into_iter()
        .chain(build_player_impact_events(&p.away_actions, &away_ctx_base))
    {
        let _ = app_event_bus.send(event.to_enveloppe());
    }

    let home_score = count_touchdowns(&p.home_actions);
    let away_score = count_touchdowns(&p.away_actions);

    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::TeamMatchConcluded {
            team_id: p.home_team_id.to_string(),
            match_report_id: home_ctx_base.match_report_id.clone(),
            round_id: home_ctx_base.round_id.clone(),
            round_label: home_ctx_base.round_label.clone(),
            opponent_team_id: home_ctx_base.opponent_team_id.clone(),
            opponent_team_name: home_ctx_base.opponent_team_name.clone(),
            team_score: home_score,
            opponent_score: away_score,
        }
        .to_enveloppe(),
    );
    let _ = app_event_bus.send(
        PlayerMatchImpactAppEvent::TeamMatchConcluded {
            team_id: p.away_team_id.to_string(),
            match_report_id: away_ctx_base.match_report_id.clone(),
            round_id: away_ctx_base.round_id.clone(),
            round_label: away_ctx_base.round_label.clone(),
            opponent_team_id: away_ctx_base.opponent_team_id.clone(),
            opponent_team_name: away_ctx_base.opponent_team_name.clone(),
            team_score: away_score,
            opponent_score: home_score,
        }
        .to_enveloppe(),
    );
}

/// Contexte commun à un camp (home ou away) — tout sauf `player_id`, résolu une
/// seule fois par publication (pas par action).
struct ContextBase {
    match_report_id: String,
    round_id: String,
    round_label: String,
    opponent_team_id: String,
    opponent_team_name: String,
}

impl ContextBase {
    fn for_player(&self, player_id: &str) -> PlayerMatchContextPayload {
        PlayerMatchContextPayload {
            match_report_id: self.match_report_id.clone(),
            round_id: self.round_id.clone(),
            round_label: self.round_label.clone(),
            opponent_team_id: self.opponent_team_id.clone(),
            opponent_team_name: self.opponent_team_name.clone(),
            player_id: player_id.to_string(),
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
        MatchActionType::Interception => {
            PlayerMatchImpactAppEvent::PlayerPerformedInterception(context)
        }
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
        InjuryType::Sequel { stat } => InjuryTypePayload::Sequel {
            stat: map_sequel_stat(*stat).to_string(),
        },
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

// ── Compensation d'une dépublication ─────────────────────────────────────────

/// Les trois enveloppes à publier : le fait lui-même, puis un ordre de
/// compensation par équipe.
///
/// Un seul événement par équipe et non un par action : chaque agrégat joueur
/// porte son propre instantané de ce que le match lui a apporté. Fonction pure,
/// donc testable sans bus ni repository.
fn build_unpublished_events(rtp: &MatchReportReadyToPublish) -> Vec<EventEnvelope> {
    let match_report_id = rtp.id.to_string();
    let home_team_id = rtp.home_team_id.to_string();
    let away_team_id = rtp.away_team_id.to_string();

    let mut events = vec![
        MatchReportAppEvent::MatchReportUnpublished(build_unpublished_payload(rtp)).to_enveloppe(),
    ];

    for team_id in [home_team_id, away_team_id] {
        events.push(
            PlayerMatchImpactAppEvent::TeamMatchImpactReverted {
                team_id,
                match_report_id: match_report_id.clone(),
            }
            .to_enveloppe(),
        );
    }
    events
}

fn build_unpublished_payload(rtp: &MatchReportReadyToPublish) -> MatchReportUnpublishedPayload {
    MatchReportUnpublishedPayload {
        match_report_id: rtp.id.to_string(),
        space_id: rtp.space_id.to_string(),
        competition_id: rtp.competition_id.to_string(),
        season_id: rtp.season_id.to_string(),
        round_id: rtp.round_id.to_string(),
        pairing_id: rtp.pairing_id.clone(),
        home_team_id: rtp.home_team_id.to_string(),
        away_team_id: rtp.away_team_id.to_string(),
        unpublished_at: chrono::Utc::now(),
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
        home_inducement_spending_kpo: p.home_inducement_spending.into_inner(),
        away_inducement_spending_kpo: p.away_inducement_spending.into_inner(),
        home_fan_mod: p.home_fan_mod.into_inner(),
        away_fan_mod: p.away_fan_mod.into_inner(),
        home_actions: build_action_payloads(&p.home_actions, &p.home_temp_players),
        away_actions: build_action_payloads(&p.away_actions, &p.away_temp_players),
        home_temp_players: p
            .home_temp_players
            .iter()
            .map(build_temp_player_payload)
            .collect(),
        away_temp_players: p
            .away_temp_players
            .iter()
            .map(build_temp_player_payload)
            .collect(),
    }
}

fn count_touchdowns(actions: &[MatchAction]) -> u8 {
    actions
        .iter()
        .filter(|a| matches!(a.action, MatchActionType::Touchdown))
        .count() as u8
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
        ActionPlayer::Regular(player_id) => PlayerRefPayload::Regular {
            player_id: player_id.to_string(),
        },
        ActionPlayer::Temp(temp_id) => temp_players
            .iter()
            .find(|t| &t.id == temp_id)
            .map(|t| match &t.kind {
                TempPlayerKind::StarPlayer { ref_uid, .. } => PlayerRefPayload::Star {
                    ref_uid: ref_uid.clone(),
                    display_name: t.display_name.clone().unwrap_or_default(),
                },
                TempPlayerKind::Mercenary { .. } => PlayerRefPayload::Mercenary,
                TempPlayerKind::Journeyman { .. } => PlayerRefPayload::Journeyman,
            })
            .unwrap_or(PlayerRefPayload::Journeyman),
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
        MatchActionType::Blesse { injury } => ActionTypePayload::Blesse {
            injury: injury_label(injury),
        },
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
        TempPlayerKind::Journeyman { .. } => "Journeyman",
    };
    TempPlayerPayload {
        id: t.id.0.clone(),
        kind: kind.to_string(),
        display_name: t.display_name.clone(),
    }
}

#[cfg(test)]
mod unpublished_events_tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        ActionId, DedicatedFans, FanFactorMod, MatchGain, MatchReportOrigin, TurnNumber,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, PlayerId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    /// Une action quelconque, pour vérifier qu'elle **ne** se retrouve **pas**
    /// dans le payload de compensation.
    fn une_action() -> MatchAction {
        MatchAction {
            id: ActionId("a1".into()),
            turn: TurnNumber::try_new(1).unwrap(),
            player: ActionPlayer::Regular(PlayerId::new()),
            action: MatchActionType::Touchdown,
            player_display_name: "Tyrandel".into(),
            player_position: "Frappeur".into(),
        }
    }

    fn rtp() -> MatchReportReadyToPublish {
        MatchReportReadyToPublish {
            id: MatchReportId::new(),
            home_inducement_spending: InducementSpending::default(),
            away_inducement_spending: InducementSpending::default(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: TeamId::new(),
            away_team_id: TeamId::new(),
            created_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: Some("pairing-1".to_string()),
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![une_action()],
            away_actions: vec![],
            version: 8,
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(5_000).unwrap(),
            home_fan_mod: FanFactorMod::try_new(1).unwrap(),
            away_fan_mod: FanFactorMod::try_new(-1).unwrap(),
            summary_title: None,
            summary_body: None,
            was_published_before: true,
        }
    }

    #[test]
    fn trois_evenements_sont_produits_un_fait_et_deux_compensations() {
        let events = build_unpublished_events(&rtp());
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "MatchReportUnpublished");
        assert_eq!(events[1].event_type, "TeamMatchImpactReverted");
        assert_eq!(events[2].event_type, "TeamMatchImpactReverted");
    }

    /// Les deux compensations visent bien les deux équipes distinctes, dans
    /// l'ordre home puis away — jamais deux fois la même.
    fn team_id_of(envelope: &EventEnvelope) -> String {
        envelope.payload["TeamMatchImpactReverted"]["team_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn une_compensation_par_equipe_sans_croisement() {
        let rtp = rtp();
        let events = build_unpublished_events(&rtp);

        assert_eq!(team_id_of(&events[1]), rtp.home_team_id.to_string());
        assert_eq!(team_id_of(&events[2]), rtp.away_team_id.to_string());
    }

    #[test]
    fn le_payload_porte_les_identifiants_du_rapport() {
        let rtp = rtp();
        let payload = build_unpublished_payload(&rtp);

        assert_eq!(payload.match_report_id, rtp.id.to_string());
        assert_eq!(payload.competition_id, rtp.competition_id.to_string());
        assert_eq!(payload.season_id, rtp.season_id.to_string());
        assert_eq!(payload.round_id, rtp.round_id.to_string());
        assert_eq!(payload.pairing_id, Some("pairing-1".to_string()));
        assert_eq!(payload.home_team_id, rtp.home_team_id.to_string());
        assert_eq!(payload.away_team_id, rtp.away_team_id.to_string());
    }

    /// Le rapport porte une action, mais le payload de compensation n'en
    /// transporte aucune : chaque BC défait ce qu'il a enregistré lui-même.
    #[test]
    fn le_payload_ne_transporte_aucune_action() {
        let events = build_unpublished_events(&rtp());
        let payload = &events[0].payload["MatchReportUnpublished"];

        assert!(payload.get("home_actions").is_none());
        assert!(payload.get("away_actions").is_none());
        assert!(payload.get("home_score").is_none());
    }
}

#[cfg(test)]
mod player_impact_tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{ActionId, TempPlayerId, TurnNumber};
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId as SharedPlayerId;

    fn ctx_base() -> ContextBase {
        ContextBase {
            match_report_id: "mr1".into(),
            round_id: "r1".into(),
            round_label: "Journée 5".into(),
            opponent_team_id: "opponent".into(),
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
        let events =
            build_player_impact_events(&[regular_action(MatchActionType::Touchdown)], &ctx_base());
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            PlayerMatchImpactAppEvent::PlayerPerformedTouchdown(_)
        ));
    }

    #[test]
    fn passe_and_lancer_both_map_to_player_performed_pass() {
        let events = build_player_impact_events(
            &[
                regular_action(MatchActionType::Passe),
                regular_action(MatchActionType::Lancer),
            ],
            &ctx_base(),
        );
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .all(|e| matches!(e, PlayerMatchImpactAppEvent::PlayerPerformedPass(_))));
    }

    #[test]
    fn agression_maps_to_foul_without_spp_bearing_variant() {
        let events =
            build_player_impact_events(&[regular_action(MatchActionType::Agression)], &ctx_base());
        assert!(matches!(
            events[0],
            PlayerMatchImpactAppEvent::PlayerPerformedFoul(_)
        ));
    }

    #[test]
    fn blesse_sequel_maps_to_player_injured_with_structured_stat() {
        let events = build_player_impact_events(
            &[regular_action(MatchActionType::Blesse {
                injury: InjuryType::Sequel {
                    stat: SequelStat::MinusAg,
                },
            })],
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
        let events =
            build_player_impact_events(&[temp_action(MatchActionType::Touchdown)], &ctx_base());
        assert!(events.is_empty());
    }
}
