use crate::app::shared_kernel::bloodbowl::ids::RosterId;
use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use crate::app::team_creation::domain::team_staff::TeamStaff;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

pub const MAX_PLAYER_COUNT: u8 = 16;

// ── IDs et plain newtypes ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialRuleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillId(pub String);

// ── Value objects ─────────────────────────────────────────────────────────────

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PlayerName(String);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct PlayerMaxQuantity(u8);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 300),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct PlayerPrice(u32);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct RerollBasePrice(u32);

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct RosterName(String);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 99),
    derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)
)]
pub struct JerseyNumber(u8);

#[nutype(
    validate(less_or_equal = 99),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppCost(u8);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct CrossLimitCount(u32);

// ── Compétences acquises ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode {
    Chosen,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkill {
    pub skill_id: SkillId,
    pub mode: AcquisitionMode,
    pub spp_cost: SppCost,
}

// ── Joueur ────────────────────────────────────────────────────────────────────

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
pub struct HiredPlayer {
    pub instance_id: PlayerId,
    pub definition: PlayerDefinition,
    #[serde(default)]
    pub personal_name: String, // arch:ok texte libre
    #[serde(default)]
    pub jersey: Option<JerseyNumber>,
    #[serde(default)]
    pub acquired_skills: Vec<AcquiredSkill>,
}

impl HiredPlayer {
    pub fn new(definition: PlayerDefinition) -> Self {
        Self {
            instance_id: PlayerId(Ulid::new().to_string()),
            definition,
            personal_name: String::new(),
            jersey: None,
            acquired_skills: Vec::new(),
        }
    }

    pub fn acquired_skill_ids_csv(&self) -> String {
        self.acquired_skills
            .iter()
            .map(|s| s.skill_id.0.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

// ── Limites croisées ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLimit {
    pub limit: CrossLimitCount,
    pub limited_player_ids: Vec<PlayerId>,
}

impl CrossLimit {
    pub fn includes_player(&self, player_id: &PlayerId) -> bool {
        self.limited_player_ids.contains(player_id)
    }
}

// ── Roster ────────────────────────────────────────────────────────────────────

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
