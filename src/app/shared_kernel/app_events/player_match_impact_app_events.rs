use crate::common::event_envelope::EventEnvelope;
use crate::app::shared_kernel::common_types::EventId;
use serde::{Deserialize, Serialize};

/// Contexte commun embarqué dans chaque event — suffisant pour reconstruire
/// l'historique du joueur côté BC `players` sans aucun appel inter-BC en lecture.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerMatchContextPayload {
    pub match_report_id:    String,
    pub round_id:           String,
    pub round_label:        String,
    pub opponent_team_id:   String,
    pub opponent_team_name: String,
    pub player_id:          String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum InjuryTypePayload {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: String },
    Mort,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PlayerMatchImpactAppEvent {
    PlayerPerformedTouchdown(PlayerMatchContextPayload),
    PlayerPerformedPass(PlayerMatchContextPayload),
    PlayerPerformedInterception(PlayerMatchContextPayload),
    PlayerPerformedCasualty(PlayerMatchContextPayload),
    PlayerPerformedMvp(PlayerMatchContextPayload),
    PlayerPerformedFoul(PlayerMatchContextPayload),
    PlayerInjured {
        context:     PlayerMatchContextPayload,
        injury_type: InjuryTypePayload,
    },
    TeamMatchConcluded {
        team_id:            String,
        match_report_id:    String,
        round_id:           String,
        round_label:        String,
        opponent_team_id:   String,
        opponent_team_name: String,
        team_score:         u8,
        opponent_score:     u8,
    },
}

impl PlayerMatchImpactAppEvent {
    pub const PLAYER_PERFORMED_TOUCHDOWN:   &'static str = "PlayerPerformedTouchdown";
    pub const PLAYER_PERFORMED_PASS:        &'static str = "PlayerPerformedPass";
    pub const PLAYER_PERFORMED_INTERCEPTION: &'static str = "PlayerPerformedInterception";
    pub const PLAYER_PERFORMED_CASUALTY:    &'static str = "PlayerPerformedCasualty";
    pub const PLAYER_PERFORMED_MVP:         &'static str = "PlayerPerformedMvp";
    pub const PLAYER_PERFORMED_FOUL:        &'static str = "PlayerPerformedFoul";
    pub const PLAYER_INJURED:               &'static str = "PlayerInjured";
    pub const TEAM_MATCH_CONCLUDED:         &'static str = "TeamMatchConcluded";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PlayerPerformedTouchdown(_)   => Self::PLAYER_PERFORMED_TOUCHDOWN,
            Self::PlayerPerformedPass(_)        => Self::PLAYER_PERFORMED_PASS,
            Self::PlayerPerformedInterception(_) => Self::PLAYER_PERFORMED_INTERCEPTION,
            Self::PlayerPerformedCasualty(_)    => Self::PLAYER_PERFORMED_CASUALTY,
            Self::PlayerPerformedMvp(_)         => Self::PLAYER_PERFORMED_MVP,
            Self::PlayerPerformedFoul(_)        => Self::PLAYER_PERFORMED_FOUL,
            Self::PlayerInjured { .. }          => Self::PLAYER_INJURED,
            Self::TeamMatchConcluded { .. }     => Self::TEAM_MATCH_CONCLUDED,
        }
    }

    fn match_report_id(&self) -> &str {
        match self {
            Self::PlayerPerformedTouchdown(c)
            | Self::PlayerPerformedPass(c)
            | Self::PlayerPerformedInterception(c)
            | Self::PlayerPerformedCasualty(c)
            | Self::PlayerPerformedMvp(c)
            | Self::PlayerPerformedFoul(c) => &c.match_report_id,
            Self::PlayerInjured { context, .. } => &context.match_report_id,
            Self::TeamMatchConcluded { match_report_id, .. } => match_report_id,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.match_report_id().to_string(),
            event_type: self.event_type().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
