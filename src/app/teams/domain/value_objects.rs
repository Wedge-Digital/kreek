use serde::{Deserialize, Serialize};

// ── Montants ──────────────────────────────────────────────────────────────────

/// Montant en kilo-Pièces d'Or.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Kpo(pub u32);

/// Delta signé en kilo-Pièces d'Or (positif = gain, négatif = perte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KpoDelta(pub i32);

// ── Noms ──────────────────────────────────────────────────────────────────────

use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 100),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct RosterName(String);

// ── Résultat de match ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchResult {
    Win,
    Draw,
    Loss,
}

impl MatchResult {
    /// Modificateur fans dévoués : +1 victoire, 0 nul, -1 défaite.
    pub fn fan_modifier(&self) -> i8 {
        match self {
            Self::Win  =>  1,
            Self::Draw =>  0,
            Self::Loss => -1,
        }
    }
}

// ── SPP ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SppGain {
    pub player_id:  crate::app::shared_kernel::common_types::PlayerId,
    pub spp_earned: u8,
}

// ── Amélioration joueur ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerImprovement {
    NewSkill(String),
    StatBoost(Stat),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stat {
    Ma, St, Ag, Pa, Av,
}

// ── Staff ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StaffType {
    Reroll,
    Apothecary,
    Assistant,
    Cheerleader,
    FansFactor,
}

// ── Erreur couteuse ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentType {
    None,
    Minor,
    Major,
    Catastrophe,
}
