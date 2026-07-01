use crate::app::shared_kernel::common_types::EventId;
use crate::common::event_envelope::EventEnvelope;
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
}

impl MatchReportAppEvent {
    pub const MATCH_REPORT_CONFIRMED: &'static str = "MatchReportConfirmed";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::MatchReportConfirmed { .. } => Self::MATCH_REPORT_CONFIRMED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::MatchReportConfirmed { match_report_id, .. } => match_report_id.clone(),
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
