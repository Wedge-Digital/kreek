use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_tags::{EventTag, EventTagName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CompetitionsDomainEvent {
    CompetitionCreated {
        event_id: EventId,
        competition_id: CompetitionId,
        space_id: SpaceId,
        created_by: CoachId,
        name: CompetitionName,
        logo: CloudinaryImage,
        admin_ids: Vec<CoachId>,
    },
    CompetitionReady {
        event_id: EventId,
        competition_id: CompetitionId,
        space_id: SpaceId,
        finalized_by: CoachId,
    },
    PairingCreated {
        event_id: EventId,
        pairing_id: String,
        competition_id: String,
        season_id: String,
        round_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
        home_team_name: String,
        home_roster_name: String,
        home_coach_name: String,
        home_logo_url: Option<String>,
        away_team_name: String,
        away_roster_name: String,
        away_coach_name: String,
        away_logo_url: Option<String>,
        round_name: String,
        round_position: i32,
        round_date_start: Option<String>,
        round_date_end: Option<String>,
        round_day_type: String,
    },
    PairingDeleted {
        event_id: EventId,
        pairing_id: String,
    },
}

pub const COMPETITION_CREATED: &str = "CompetitionCreated";
pub const COMPETITION_READY: &str = "CompetitionReady";
pub const PAIRING_CREATED: &str = "PairingCreated";
pub const PAIRING_DELETED: &str = "PairingDeleted";

impl CompetitionsDomainEvent {
    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::CompetitionCreated { .. } => COMPETITION_CREATED,
            Self::CompetitionReady { .. } => COMPETITION_READY,
            Self::PairingCreated { .. } => PAIRING_CREATED,
            Self::PairingDeleted { .. } => PAIRING_DELETED,
        }
    }

    fn emitter_id(&self) -> String {
        match self {
            Self::CompetitionCreated { competition_id, .. } => competition_id.to_string(),
            Self::CompetitionReady { competition_id, .. } => competition_id.to_string(),
            Self::PairingCreated { pairing_id, .. } => pairing_id.clone(),
            Self::PairingDeleted { pairing_id, .. } => pairing_id.clone(),
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::CompetitionCreated {
                space_id,
                competition_id,
                ..
            }
            | Self::CompetitionReady {
                space_id,
                competition_id,
                ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::Competition,
                    value: competition_id.to_string(),
                },
            ],
            Self::PairingCreated { .. } | Self::PairingDeleted { .. } => vec![],
        }
    }

    pub fn to_app_event(&self) -> Option<CompetitionsAppEvent> {
        match self {
            Self::CompetitionReady {
                competition_id,
                space_id,
                ..
            } => Some(CompetitionsAppEvent::CompetitionCreated {
                event_id: EventId::new(),
                competition_id: *competition_id,
                space_id: *space_id,
            }),
            Self::PairingCreated {
                pairing_id,
                competition_id,
                season_id,
                round_id,
                home_team_id,
                away_team_id,
                space_id,
                ..
            } => Some(CompetitionsAppEvent::PairingCreated {
                event_id: EventId::new(),
                pairing_id: pairing_id.clone(),
                competition_id: competition_id.clone(),
                season_id: season_id.clone(),
                round_id: round_id.clone(),
                home_team_id: home_team_id.clone(),
                away_team_id: away_team_id.clone(),
                space_id: space_id.clone(),
            }),
            Self::PairingDeleted { pairing_id, .. } => Some(CompetitionsAppEvent::PairingDeleted {
                event_id: EventId::new(),
                pairing_id: pairing_id.clone(),
            }),
            _ => None,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.emitter_id(),
            event_type: self.to_event_type().to_string(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
