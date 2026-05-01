use serde::{Deserialize, Serialize};
use crate::app::team_creation::team_staff::TeamStaff;

pub const MAX_PLAYER_COUNT: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerName(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerMaxQuantity(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerPrice(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RerollBasePrice(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RosterId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLimit {
    pub limit: u32,
    pub limited_player_ids: Vec<PlayerId>,
}

impl CrossLimit {
    pub fn includes_player(&self, player_id: &PlayerId) -> bool {
        self.limited_player_ids.contains(player_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDefinition {
    pub id: PlayerId,
    pub name: PlayerName,
    pub max_quantity: PlayerMaxQuantity,
    pub price: PlayerPrice,
}

impl PartialEq for PlayerDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roster {
    pub id: RosterId,
    pub name: RosterName,
    pub player_definitions: Vec<PlayerDefinition>,
    pub allowed_staff: Vec<TeamStaff>,
    pub cross_limits: Vec<CrossLimit>,
    pub reroll_price: RerollBasePrice,
}

impl PartialEq for Roster {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Roster {
    pub fn contains_player(&self, player: &PlayerDefinition) -> bool {
        self.player_definitions.contains(player)
    }

    pub fn has_cross_limits(&self) -> bool {
        !self.cross_limits.is_empty()
    }
}