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

/// Seuil de TD marqués déclenchant le bonus offensif (≥ seuil).
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MinTd(u32);

/// Seuil de TD encaissés en-dessous duquel le bonus défensif s'applique (≤ seuil).
#[nutype(
    validate(less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MaxTdConceded(u32);

/// Seuil (strict) de sorties infligées déclenchant le bonus agressif (> seuil).
#[nutype(
    validate(less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MinCasualties(u32);

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
    #[serde(default = "default_aggressive_bonus")]
    pub aggressive_bonus: AggressiveBonus,
    pub additionnal_ranking_points: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveBonus {
    pub activated: Activated,
    #[serde(rename = "diff_td")]
    pub min_td: MinTd,
    pub points: RankingPoints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
    #[serde(default = "default_max_td_conceded")]
    pub max_td_conceded: MaxTdConceded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggressiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
    pub min_casualties: MinCasualties,
}

fn default_max_td_conceded() -> MaxTdConceded {
    MaxTdConceded::try_new(1).expect("1 est dans les bornes de MaxTdConceded")
}

fn default_aggressive_bonus() -> AggressiveBonus {
    AggressiveBonus {
        activated: Activated(false),
        points: RankingPoints::try_new(1).expect("1 est dans les bornes de RankingPoints"),
        min_casualties: MinCasualties::try_new(2).expect("2 est dans les bornes de MinCasualties"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn td_thresholds_accept_valid_and_reject_out_of_bounds() {
        assert!(MinTd::try_new(1).is_ok());
        assert!(MinTd::try_new(16).is_ok());
        assert!(MinTd::try_new(0).is_err());
        assert!(MinTd::try_new(17).is_err());

        assert!(MaxTdConceded::try_new(0).is_ok());
        assert!(MaxTdConceded::try_new(16).is_ok());
        assert!(MaxTdConceded::try_new(17).is_err());

        assert!(MinCasualties::try_new(0).is_ok());
        assert!(MinCasualties::try_new(16).is_ok());
        assert!(MinCasualties::try_new(17).is_err());
    }

    #[test]
    fn legacy_rules_without_new_fields_deserialize_with_defaults() {
        // JSON antérieur à la feature : pas de max_td_conceded ni aggressive_bonus.
        let json = r#"{
            "win_points": 3, "draw_points": 1, "lose_points": 0,
            "offensive_bonus": { "activated": true, "diff_td": 3, "points": 1 },
            "defensive_bonus": { "activated": true, "points": 2 },
            "additionnal_ranking_points": {}
        }"#;

        let rr: RankingRules = serde_json::from_str(json).unwrap();

        assert_eq!(rr.defensive_bonus.max_td_conceded, default_max_td_conceded());
        assert_eq!(rr.aggressive_bonus.activated, Activated(false));
        assert_eq!(rr.aggressive_bonus.min_casualties, MinCasualties::try_new(2).unwrap());
    }

    #[test]
    fn offensive_bonus_keeps_diff_td_json_key_for_min_td_field() {
        let json = r#"{ "activated": true, "diff_td": 5, "points": 1 }"#;
        let ob: OffensiveBonus = serde_json::from_str(json).unwrap();
        assert_eq!(ob.min_td, MinTd::try_new(5).unwrap());

        // Round-trip : la clé JSON reste "diff_td".
        let serialized = serde_json::to_string(&ob).unwrap();
        assert!(serialized.contains("\"diff_td\":5"));
        assert!(!serialized.contains("min_td"));
    }
}
