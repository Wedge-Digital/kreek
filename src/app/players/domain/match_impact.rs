use crate::app::players::domain::player::TeamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};

// ── Contexte de match (embarqué dans chaque event, zéro appel inter-BC en lecture) ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReportId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchContext {
    pub match_report_id:    MatchReportId,
    pub round_id:           RoundId,
    pub round_label:        String,
    pub opponent_team_id:   TeamId,
    pub opponent_team_name: String,
}

// ── SPP gagné par une action (résolu en amont via references, jamais calculé ici) ──

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct SppEarned(u32);

// ── Statut de participation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerParticipationStatus {
    Available,
    MissingNextGame,
    Retired,
    Dead,
}

// ── Blessures ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    Ma,
    St,
    Ag,
    Pa,
    Av,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: StatKind },
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInjuryRecord {
    pub injury_type: InjuryType,
    pub context:     MatchContext,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatAdjustment {
    pub stat:  StatKind,
    pub malus: u8,
}

// ── Compteurs de carrière (même style que Spp/ValueKpo dans player.rs) ─────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TouchdownCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PassCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InterceptionCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CasualtyCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MvpCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoulCount(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersistentInjuryCount(pub u16);
