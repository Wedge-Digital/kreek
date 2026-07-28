use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::match_impact::{
    InjuryType, MatchContext, MatchReportId, SppEarned, StatKind,
};
use crate::app::players::domain::value_objects::{
    JerseyVo, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::shared_kernel::identity::ids::SpaceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDomainEvent {
    PlayerCreated {
        player_id:      PlayerId,
        team_id:        TeamId,
        space_id:       SpaceId,
        position_name:  PositionNameVo,
        roster_line_id: RosterLineId,
        jersey:         Option<JerseyVo>,
        base_skills:    Vec<SkillId>,
        starting_spp:   Spp,
        starting_value: ValueKpo,
    },
    InitialSkillEarned {
        player_id:    PlayerId,
        team_id:      TeamId,
        skill_id:     SkillId,
        skill_name:   SkillName,
        category_css: String,
        mode:         AcquisitionMode,
        spp_cost:     SppCost,
        is_primary:   bool,
        is_elite:     bool,
        value_delta:  ValueKpo,
    },

    // ── Dépense de SPP post-match (phase PlayerImprovement) ────────────────────
    PlayerSkillPurchased {
        player_id:    PlayerId,
        team_id:      TeamId,
        skill_id:     SkillId,
        skill_name:   SkillName,
        category_css: String,
        mode:         AcquisitionMode,
        spp_cost:     SppCost,
        value_delta:  ValueKpo,
    },
    PlayerStatIncreased {
        player_id:   PlayerId,
        team_id:     TeamId,
        stat:        StatKind,
        spp_cost:    SppCost,
        value_delta: ValueKpo,
    },

    // ── Impact des rapports de match ───────────────────────────────────────────
    // player_id/team_id sont redondants avec l'agrégat (déjà identifié par son
    // propre flux d'events) mais nécessaires à la couche persistance, qui route
    // l'append par (player_id, team_id) — même besoin que PlayerCreated/InitialSkillEarned.
    TouchdownScored {
        player_id:  PlayerId,
        team_id:    TeamId,
        context:    MatchContext,
        spp_earned: SppEarned,
    },
    PassCompleted {
        player_id:  PlayerId,
        team_id:    TeamId,
        context:    MatchContext,
        spp_earned: SppEarned,
    },
    InterceptionMade {
        player_id:  PlayerId,
        team_id:    TeamId,
        context:    MatchContext,
        spp_earned: SppEarned,
    },
    CasualtyInflicted {
        player_id:  PlayerId,
        team_id:    TeamId,
        context:    MatchContext,
        spp_earned: SppEarned,
    },
    MatchMvpNamed {
        player_id:  PlayerId,
        team_id:    TeamId,
        context:    MatchContext,
        spp_earned: SppEarned,
    },
    FoulCommitted {
        player_id: PlayerId,
        team_id:   TeamId,
        context:   MatchContext,
    },
    InjurySustained {
        player_id:   PlayerId,
        team_id:     TeamId,
        context:     MatchContext,
        injury_type: InjuryType,
    },
    PlayerAvailabilityRestored {
        player_id:       PlayerId,
        team_id:         TeamId,
        match_report_id: MatchReportId,
    },
    MatchConcluded {
        player_id:      PlayerId,
        team_id:        TeamId,
        context:        MatchContext,
        team_score:     u8,
        opponent_score: u8,
    },

    /// L'impact de ce match sur ce joueur a été annulé — le rapport a été
    /// dépublié pour correction.
    ///
    /// Événement **mince** à dessein : il énonce un fait, pas les montants à
    /// retrancher. Ceux-ci vivent dans l'instantané `last_match` de l'agrégat,
    /// lui-même reconstruit par les événements qui précèdent. Au rejeu, `apply`
    /// dispose donc exactement des mêmes valeurs qu'au moment de l'émission.
    MatchImpactReverted {
        player_id:       PlayerId,
        team_id:         TeamId,
        match_report_id: MatchReportId,
    },
}

impl PlayerDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::PlayerCreated { .. } => "PlayerCreated",
            Self::InitialSkillEarned { .. } => "InitialSkillEarned",
            Self::PlayerSkillPurchased { .. } => "PlayerSkillPurchased",
            Self::PlayerStatIncreased { .. } => "PlayerStatIncreased",
            Self::TouchdownScored { .. } => "TouchdownScored",
            Self::PassCompleted { .. } => "PassCompleted",
            Self::InterceptionMade { .. } => "InterceptionMade",
            Self::CasualtyInflicted { .. } => "CasualtyInflicted",
            Self::MatchMvpNamed { .. } => "MatchMvpNamed",
            Self::FoulCommitted { .. } => "FoulCommitted",
            Self::InjurySustained { .. } => "InjurySustained",
            Self::PlayerAvailabilityRestored { .. } => "PlayerAvailabilityRestored",
            Self::MatchConcluded { .. } => "MatchConcluded",
            Self::MatchImpactReverted { .. } => "MatchImpactReverted",
        }
    }

    /// Publication sur le bus interne du BC (pas l'event store — cf.
    /// `IPlayerRepository::append` pour la persistance). Seuls les use cases
    /// qui doivent notifier un autre BC (ex. dépense de SPP → `teams`)
    /// publient sur ce bus.
    pub fn to_enveloppe(&self, player_id: &str) -> crate::common::event_envelope::EventEnvelope {
        crate::common::event_envelope::EventEnvelope {
            event_id: crate::app::shared_kernel::identity::ids::EventId::new().to_string(),
            emitter: player_id.to_string(),
            event_type: self.type_name().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }

    /// Conversion vers l'app event franchissant la frontière vers `teams` —
    /// `None` pour tout ce qui ne concerne pas `team_value` (la plupart des
    /// events). Seul le publisher (couche IO, `app_event_publisher.rs`)
    /// appelle cette méthode.
    pub fn to_app_event(
        &self,
    ) -> Option<crate::app::shared_kernel::app_events::player_improvement_app_events::PlayerImprovementAppEvent> {
        use crate::app::shared_kernel::app_events::player_improvement_app_events::PlayerImprovementAppEvent;
        match self {
            Self::PlayerSkillPurchased { team_id, player_id, skill_name, value_delta, .. } => {
                Some(PlayerImprovementAppEvent::SkillPurchased {
                    team_id: team_id.0.clone(),
                    player_id: player_id.0.clone(),
                    skill_name: skill_name.to_string(),
                    value_delta_po: value_delta.0,
                })
            }
            Self::PlayerStatIncreased { team_id, player_id, stat, value_delta, .. } => {
                Some(PlayerImprovementAppEvent::StatIncreased {
                    team_id: team_id.0.clone(),
                    player_id: player_id.0.clone(),
                    stat: stat_name(*stat).to_string(),
                    value_delta_po: value_delta.0,
                })
            }
            _ => None,
        }
    }
}

fn stat_name(stat: StatKind) -> &'static str {
    match stat {
        StatKind::Ma => "Ma",
        StatKind::St => "St",
        StatKind::Ag => "Ag",
        StatKind::Pa => "Pa",
        StatKind::Av => "Av",
    }
}
