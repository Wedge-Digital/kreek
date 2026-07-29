use serde::Deserialize;

// ── Inducement ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Inducement {
    pub uid: String,
    pub name: String,
    pub cost: u32,
    #[serde(rename = "reducedCost")]
    pub reduced_cost: Option<u32>,
    #[serde(rename = "maxQuantity")]
    pub max_quantity: u32,
    pub category: String,
    #[serde(rename = "restrictedTo", default)]
    pub restricted_to: Vec<String>,
    pub description: String,
}

// ── StarPlayer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct StarPlayer {
    pub uid: String,
    pub name: String,
    pub cost: u32,
    #[serde(rename = "MA")]
    pub ma: u8,
    #[serde(rename = "ST")]
    pub st: u8,
    #[serde(rename = "AG")]
    pub ag: String,
    #[serde(rename = "PA")]
    pub pa: String,
    #[serde(rename = "AV")]
    pub av: String,
    #[serde(rename = "playerType")]
    pub player_type: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(rename = "specialAbilityName")]
    pub special_ability_name: String,
    #[serde(rename = "specialAbilityDescription")]
    pub special_ability_description: String,
    #[serde(rename = "playsFor", default)]
    pub plays_for: Vec<String>,
    #[serde(rename = "availableForRosters", default)]
    pub available_for_rosters: Vec<String>,
}

// ── Team ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerPosition {
    pub uid: String,
    #[serde(rename = "positionName")]
    pub position_name: String,
    pub cost: u32,
    #[serde(rename = "MA")]
    pub ma: u8,
    #[serde(rename = "ST")]
    pub st: u8,
    #[serde(rename = "AG")]
    pub ag: u8,
    #[serde(rename = "PA")]
    pub pa: u8,
    #[serde(rename = "AV")]
    pub av: u8,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(rename = "primaryAccess", default)]
    pub primary_access: Vec<String>,
    #[serde(rename = "secondaryAccess", default)]
    pub secondary_access: Vec<String>,
    #[serde(rename = "max_quantity")]
    pub max_quantity: u8,
    #[serde(default)]
    pub is_journeyman: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Team {
    pub uid: String,
    pub name: String,
    #[serde(rename = "rerollCost")]
    pub reroll_cost: u32,
    pub tier: String,
    #[serde(rename = "specialRules", default)]
    pub special_rules: Vec<String>,
    #[serde(rename = "allowedStaff", default)]
    pub allowed_staff: Vec<String>,
    #[serde(rename = "availablePlayers", default)]
    pub available_players: Vec<PlayerPosition>,
    #[serde(default)]
    pub leagues: Vec<String>,
    #[serde(default)]
    pub logo: Option<String>,
}

// ── SkillCost ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ChosenSkillCost {
    pub primary: u8,
    pub secondary: u8,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct SkillCostLevel {
    pub level: u8,
    pub chosen: ChosenSkillCost,
    #[serde(rename = "chosenElite", default)]
    pub chosen_elite: Option<ChosenSkillCost>,
    pub random: u8,
    #[serde(rename = "randomElite", default)]
    pub random_elite: Option<u8>,
    pub characteristic: u8,
}

impl SkillCostLevel {
    pub fn chosen_for(&self, is_elite: bool) -> &ChosenSkillCost {
        if is_elite {
            self.chosen_elite.as_ref().unwrap_or(&self.chosen)
        } else {
            &self.chosen
        }
    }

    pub fn random_for(&self, is_elite: bool) -> u8 {
        if is_elite {
            self.random_elite.unwrap_or(self.random)
        } else {
            self.random
        }
    }
}

// ── Skill ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub uid: String,
    pub name: String,
    pub category: String,
    #[serde(rename = "type")]
    pub skill_type: String,
    pub activation: String,
    pub description: String,
}

// ── SkillCategory ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SkillCategory {
    pub id: String,
    pub label: String,
}

// ── SpecialRule ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SpecialRule {
    pub uid: String,
    pub label: String,
}

// ── Staff ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Staff {
    pub uid: String,
    pub name: String,
    pub price: u32,
    #[serde(rename = "maxQuantity")]
    pub max_quantity: u32,
    pub description: String,
}

// ── League ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct League {
    pub uid: String,
    pub label: String,
}
