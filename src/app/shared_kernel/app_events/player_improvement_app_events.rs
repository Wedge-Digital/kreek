use crate::app::shared_kernel::common_types::EventId;
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Franchit la frontière `players` → `teams` quand un joueur achète une
/// compétence ou une augmentation de caractéristique en SPP (phase
/// PlayerImprovement). `value_delta_po` est exprimé en **Po** (cohérent avec
/// `players::domain::player::ValueKpo`, malgré son nom) — le consommateur
/// (`teams`) doit diviser par 1000 avant de construire son propre `Kpo`, qui
/// lui stocke déjà des kPo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerImprovementAppEvent {
    SkillPurchased {
        team_id: String,
        player_id: String,
        skill_name: String,
        value_delta_po: u32,
    },
    StatIncreased {
        team_id: String,
        player_id: String,
        stat: String,
        value_delta_po: u32,
    },
}

impl PlayerImprovementAppEvent {
    pub const SKILL_PURCHASED: &'static str = "PlayerImprovementSkillPurchased";
    pub const STAT_INCREASED: &'static str = "PlayerImprovementStatIncreased";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SkillPurchased { .. } => Self::SKILL_PURCHASED,
            Self::StatIncreased { .. } => Self::STAT_INCREASED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::SkillPurchased { player_id, .. } => player_id.clone(),
            Self::StatIncreased { player_id, .. } => player_id.clone(),
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
