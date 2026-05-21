use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionRules {
    pub ranking_rules: RankingRules,
    pub tiers:         Vec<TierRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRules {
    pub win_points:                 u32,
    pub draw_points:                u32,
    pub lose_points:                u32,
    pub offensive_bonus:            OffensiveBonus,
    pub defensive_bonus:            DefensiveBonus,
    pub additionnal_ranking_points: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveBonus {
    pub activated: bool,
    pub diff_td:   u32,
    pub points:    u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensiveBonus {
    pub activated: bool,
    pub points:    u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRule {
    pub name:        String,
    pub budget:      u32,
    pub starting_xp: u32,
    pub rosters:     Vec<String>,
    pub inducements: Vec<String>,
    pub star_players: Vec<String>,
}
