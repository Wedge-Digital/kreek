use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
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
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct RosterName(String);

/// Identifiant opaque d'un roster externe (BC references). Pas un ULID.
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 100),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct RosterRef(String);

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 100, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct TeamName(String);

// ── Résultat de match ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchResult {
    Win,
    Draw,
    Loss,
}

// ── Fans dévoués ──────────────────────────────────────────────────────────────

#[nutype(
    validate(less_or_equal = 20),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct DedicatedFans(u8);

// ── Quantité staff ────────────────────────────────────────────────────────────

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct StaffQuantity(u8);

// ── SPP ───────────────────────────────────────────────────────────────────────

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppEarned(u8);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SppGain {
    pub player_id: crate::app::shared_kernel::bloodbowl::ids::PlayerId,
    pub spp_earned: SppEarned,
}

// ── Amélioration joueur ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerImprovement {
    NewSkill(String),
    StatBoost(Stat),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stat {
    Ma,
    St,
    Ag,
    Pa,
    Av,
}

// ── Staff ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_name_valid() {
        assert!(TeamName::try_new("Les Korrigans FC".to_string()).is_ok());
        assert!(TeamName::try_new("Skaven Rats".to_string()).is_ok());
        assert!(TeamName::try_new("A".to_string()).is_ok());
    }

    #[test]
    fn team_name_empty_invalid() {
        assert!(TeamName::try_new("".to_string()).is_err());
        assert!(TeamName::try_new("   ".to_string()).is_err());
    }

    #[test]
    fn team_name_too_long_invalid() {
        let long = "A".repeat(101);
        assert!(TeamName::try_new(long).is_err());
    }

    /// Depuis le charset commun, la ponctuation passe — apostrophes et
    /// tirets des deux formes compris, ce qui était le but de l'opération.
    #[test]
    fn team_name_punctuation_is_allowed() {
        assert!(TeamName::try_new("Les@Korrigans".to_string()).is_ok());
        assert!(TeamName::try_new("Team!".to_string()).is_ok());
        assert!(TeamName::try_new("L'Équipe d’Or".to_string()).is_ok());
        assert!(TeamName::try_new("Korrigans — Réserve".to_string()).is_ok());
    }

    #[test]
    fn team_name_invalid_chars() {
        assert!(TeamName::try_new("Les|Korrigans".to_string()).is_err());
        assert!(TeamName::try_new("<script>".to_string()).is_err());
    }

    #[test]
    fn dedicated_fans_valid() {
        assert!(DedicatedFans::try_new(0).is_ok());
        assert!(DedicatedFans::try_new(10).is_ok());
        assert!(DedicatedFans::try_new(20).is_ok());
    }

    #[test]
    fn dedicated_fans_exceeds_max() {
        assert!(DedicatedFans::try_new(21).is_err());
    }

    #[test]
    fn staff_quantity_valid() {
        assert!(StaffQuantity::try_new(1).is_ok());
        assert!(StaffQuantity::try_new(4).is_ok());
    }

    #[test]
    fn staff_quantity_zero_invalid() {
        assert!(StaffQuantity::try_new(0).is_err());
    }

    #[test]
    fn spp_earned_valid() {
        assert!(SppEarned::try_new(1).is_ok());
        assert!(SppEarned::try_new(6).is_ok());
    }

    #[test]
    fn spp_earned_zero_invalid() {
        assert!(SppEarned::try_new(0).is_err());
    }

    #[test]
    fn roster_name_valid() {
        assert!(RosterName::try_new("Elfes Sylvestres".to_string()).is_ok());
    }

    #[test]
    fn roster_name_empty_invalid() {
        assert!(RosterName::try_new("".to_string()).is_err());
    }
}
