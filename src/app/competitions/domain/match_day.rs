use crate::app::shared_kernel::common_types::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::team::TeamId;
use crate::app::shared_kernel::date_string::DateString;
use crate::app::shared_kernel::name_vo::NameVo;
use nutype::nutype;
use std::collections::HashSet;

pub type MatchDayName = NameVo;

#[nutype(
    validate(greater_or_equal = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Display)
)]
pub struct MatchDayPosition(i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDayType {
    FixedDate,
    TimeFrame,
    Rest,
}

impl MatchDayType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FixedDate => "fixed_date",
            Self::TimeFrame => "time_frame",
            Self::Rest => "rest",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fixed_date" => Self::FixedDate,
            "rest" => Self::Rest,
            _ => Self::TimeFrame,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pairing {
    pub id: PairingId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
}

#[derive(Debug, Clone)]
pub struct MatchDay {
    pub id: MatchId,
    pub season_id: SeasonId,
    pub name: MatchDayName,
    pub day_type: MatchDayType,
    pub date_start: Option<DateString>,
    pub date_end: Option<DateString>,
    pub position: MatchDayPosition,
    pub pairings: Vec<Pairing>,
}

impl MatchDay {
    pub fn is_rest(&self) -> bool {
        self.day_type == MatchDayType::Rest
    }
}

fn normalize_pair(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

pub fn generate_round_pairings(
    teams: &[String],
    already_played: &HashSet<(String, String)>,
) -> Vec<(String, String)> {
    if teams.len() < 2 {
        return vec![];
    }

    let all_pairs: Vec<(String, String)> = {
        let mut pairs = Vec::new();
        for i in 0..teams.len() {
            for j in (i + 1)..teams.len() {
                pairs.push(normalize_pair(&teams[i], &teams[j]));
            }
        }
        pairs
    };

    let mut available: Vec<&(String, String)> = all_pairs
        .iter()
        .filter(|p| !already_played.contains(*p))
        .collect();

    if available.is_empty() {
        available = all_pairs.iter().collect();
    }

    let mut result = Vec::new();
    let mut used: HashSet<&str> = HashSet::new();

    for pair in &available {
        if !used.contains(pair.0.as_str()) && !used.contains(pair.1.as_str()) {
            used.insert(&pair.0);
            used.insert(&pair.1);
            result.push((pair.0.clone(), pair.1.clone()));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teams(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("team-{}", i + 1)).collect()
    }

    #[test]
    fn four_teams_no_played_generates_two_pairings() {
        let t = teams(4);
        let played = HashSet::new();
        let result = generate_round_pairings(&t, &played);
        assert_eq!(result.len(), 2);
        let mut used: HashSet<&str> = HashSet::new();
        for (a, b) in &result {
            assert!(used.insert(a));
            assert!(used.insert(b));
        }
    }

    #[test]
    fn four_teams_two_played_generates_different_pairings() {
        let t = teams(4);
        let mut played = HashSet::new();
        let first = generate_round_pairings(&t, &played);
        assert_eq!(first.len(), 2);

        for p in &first {
            played.insert(p.clone());
        }

        let second = generate_round_pairings(&t, &played);
        assert_eq!(second.len(), 2);
        for p in &second {
            assert!(!first.contains(p), "paire répétée : {:?}", p);
        }
    }

    #[test]
    fn four_teams_all_played_restarts_cycle() {
        let t = teams(4);
        let mut played = HashSet::new();

        for _ in 0..3 {
            let round = generate_round_pairings(&t, &played);
            for p in &round {
                played.insert(p.clone());
            }
        }

        assert_eq!(played.len(), 6);
        let next = generate_round_pairings(&t, &played);
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn three_teams_generates_one_pairing() {
        let t = teams(3);
        let played = HashSet::new();
        let result = generate_round_pairings(&t, &played);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn zero_or_one_team_generates_nothing() {
        assert!(generate_round_pairings(&[], &HashSet::new()).is_empty());
        assert!(generate_round_pairings(&teams(1), &HashSet::new()).is_empty());
    }

    #[test]
    fn normalization_treats_ab_and_ba_as_same() {
        let t = teams(4);
        let mut played = HashSet::new();
        played.insert(normalize_pair("team-2", "team-1"));

        let result = generate_round_pairings(&t, &played);
        for (a, b) in &result {
            let norm = normalize_pair(a, b);
            assert_ne!(norm, normalize_pair("team-1", "team-2"));
        }
    }
}
