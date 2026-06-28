use crate::app::shared_kernel::tier::{CreationBudget, StartingXp, TierName};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Activated(pub bool);

#[nutype(
    validate(less_or_equal = 100_000),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct RankingPoints(u32);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct TdDiff(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionRules {
    pub ranking_rules: RankingRules,
    pub tiers: Vec<TierRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRules {
    pub win_points: RankingPoints,
    pub draw_points: RankingPoints,
    pub lose_points: RankingPoints,
    pub offensive_bonus: OffensiveBonus,
    pub defensive_bonus: DefensiveBonus,
    pub additionnal_ranking_points: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveBonus {
    pub activated: Activated,
    pub diff_td: TdDiff,
    pub points: RankingPoints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRule {
    pub name: TierName,
    pub budget: CreationBudget,
    pub starting_xp: StartingXp,
    pub rosters: Vec<String>,
    pub inducements: Vec<String>,
    pub star_players: Vec<String>,
}
