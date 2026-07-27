use crate::app::shared_kernel::common_types::EventId;
use crate::common::event_envelope::EventEnvelope;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MatchReportAppEvent {
    MatchReportConfirmed {
        event_id: EventId,
        match_report_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
        pairing_id: Option<String>,
    },
    MatchReportPublished(MatchReportPublishedPayload),
    /// Le rapport repasse en état corrigeable : les BCs qui en avaient tiré des
    /// conséquences doivent les défaire.
    MatchReportUnpublished(MatchReportUnpublishedPayload),
}

/// Identifiants seulement, **aucune action** : chaque BC défait ce qu'il a
/// lui-même enregistré, via son propre instantané. Il ne recalcule rien depuis
/// ce payload — c'est ce qui rend la compensation exacte même si le payload
/// dérivait.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchReportUnpublishedPayload {
    pub match_report_id: String,
    pub space_id:        String,
    pub competition_id:  String,
    pub season_id:       String,
    pub round_id:        String,
    pub pairing_id:      Option<String>,
    pub home_team_id:    String,
    pub away_team_id:    String,
    pub unpublished_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchReportPublishedPayload {
    pub match_report_id: String,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub pairing_id: Option<String>,
    pub published_at: DateTime<Utc>,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_score: u8,
    pub away_score: u8,
    pub home_gain_kpo: u32,
    pub away_gain_kpo: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub home_actions: Vec<MatchActionPublishedPayload>,
    pub away_actions: Vec<MatchActionPublishedPayload>,
    pub home_temp_players: Vec<TempPlayerPayload>,
    pub away_temp_players: Vec<TempPlayerPayload>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchActionPublishedPayload {
    pub turn: u8,
    pub player: PlayerRefPayload,
    pub action: ActionTypePayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PlayerRefPayload {
    Regular { player_id: String },
    Star { ref_uid: String, display_name: String },
    Mercenary,
    Journeyman,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ActionTypePayload {
    Touchdown,
    Passe,
    Interception,
    Agression,
    Lancer,
    Sortie,
    Mvp,
    Blesse { injury: String },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TempPlayerPayload {
    pub id: String,
    pub kind: String,
    pub display_name: Option<String>,
}

impl MatchReportAppEvent {
    pub const MATCH_REPORT_CONFIRMED: &'static str = "MatchReportConfirmed";
    pub const MATCH_REPORT_PUBLISHED: &'static str = "MatchReportPublished";
    pub const MATCH_REPORT_UNPUBLISHED: &'static str = "MatchReportUnpublished";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::MatchReportConfirmed { .. } => Self::MATCH_REPORT_CONFIRMED,
            Self::MatchReportPublished(_) => Self::MATCH_REPORT_PUBLISHED,
            Self::MatchReportUnpublished(_) => Self::MATCH_REPORT_UNPUBLISHED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::MatchReportConfirmed { match_report_id, .. } => match_report_id.clone(),
            Self::MatchReportPublished(payload) => payload.match_report_id.clone(),
            Self::MatchReportUnpublished(payload) => payload.match_report_id.clone(),
        };
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter,
            event_type: self.event_type().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
