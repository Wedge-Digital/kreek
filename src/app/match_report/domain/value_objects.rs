use crate::app::match_report::domain::error::DomainError;
use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RosterPositionUid(pub String);

impl RosterPositionUid {
    pub fn try_new(s: &str) -> Result<Self, &'static str> {
        if s.is_empty() { Err("position uid vide") } else { Ok(Self(s.to_string())) }
    }
}

impl fmt::Display for RosterPositionUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsStarPlayer(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchReportOrigin {
    Manual,
    Pairing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct D3Roll(u8);

impl D3Roll {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=3).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidD3Roll(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

#[nutype(
    validate(less_or_equal = 3000),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Display)
)]
pub struct TeamValue(u32);

/// Copie du `teams::DedicatedFans` de chaque équipe au moment de l'enregistrement
/// du facteur fans — même borne (≤20), dupliquée ici car `match_report` ne peut
/// pas importer le type domaine de `teams` (règle de souveraineté des BCs).
#[nutype(
    validate(less_or_equal = 20),
    default = 0,
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)
)]
pub struct DedicatedFans(u32);

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 10),
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct InducementQty(u8);

#[nutype(
    validate(greater_or_equal = 1),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct InducementCost(u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InducementPurchase {
    pub uid: InducementId,
    pub qty: InducementQty,
    pub unit_cost: InducementCost,
}

impl InducementPurchase {
    pub fn total_cost(&self) -> u32 {
        self.unit_cost.into_inner() * self.qty.into_inner() as u32
    }
}

#[derive(Debug, Clone)]
pub struct AllowedInducementSpec {
    pub uid: InducementId,
    pub max_qty: InducementQty,
    pub unit_cost: InducementCost,
    pub is_star_player: IsStarPlayer,
}

// ── step5 : après-match ───────────────────────────────────────────────────────

#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct MatchGain(u32);

#[nutype(
    validate(greater_or_equal = -2, less_or_equal = 2),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct FanFactorMod(i8);

// ── step3-4 : actions match ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TempPlayerId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnNumber(u8);

impl TurnNumber {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=16).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidTurn(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamSide {
    Home,
    Away,
}

/// Verdict sur la possibilité de corriger un rapport déjà publié.
///
/// Calculé hors du domaine — il dépend de l'état d'autres BCs (phase de jeu des
/// équipes, SPP déjà dépensés) — puis remis au domaine, seul habilité à décider
/// si la dépublication est permise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionEligibility {
    Eligible,
    Blocked(CorrectionBlocker),
}

/// Ce qui empêche la correction. Porte un `TeamSide` et non un nom d'équipe :
/// le domaine ignore les chaînes d'affichage, la résolution du nom appartient à
/// la couche de présentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionBlocker {
    /// Un joueur de ce camp a dépensé des SPP depuis le match.
    SppAlreadySpent { side: TeamSide },
    /// Ce camp a quitté la phase d'amélioration des joueurs.
    PhaseAdvanced { side: TeamSide },
    /// Une des consultations nécessaires au verdict n'a pas abouti. On échoue
    /// fermé : autoriser une correction qui aurait dû être refusée laisserait
    /// des données incohérentes, alors qu'un refus indu ne fait que retarder.
    EligibilityUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPlayer {
    Regular(PlayerId),
    Temp(TempPlayerId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchActionType {
    Touchdown,
    Passe,
    Interception,
    Agression,
    Lancer,
    Sortie,
    Mvp,
    Blesse { injury: InjuryType },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: SequelStat },
    Mort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequelStat {
    MinusAv,
    MinusMa,
    MinusPa,
    MinusAg,
    MinusSt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempPlayer {
    pub id: TempPlayerId,
    pub team_id: TeamId,
    pub kind: TempPlayerKind,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempPlayerKind {
    StarPlayer { ref_uid: String, position_uid: String },
    Mercenary { position_uid: String },
    Journeyman { position_uid: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAction {
    pub id: ActionId,
    pub turn: TurnNumber,
    pub player: ActionPlayer,
    pub action: MatchActionType,
    pub player_display_name: String, // arch:ok texte libre dénormalisé (snapshot pour l'historique)
    #[serde(default)]
    pub player_position: String, // arch:ok texte libre dénormalisé (snapshot pour l'historique)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_value_ord() {
        let a = TeamValue::try_new(1000).unwrap();
        let b = TeamValue::try_new(1100).unwrap();
        assert!(b > a);
        assert!(a < b);
        assert_eq!(TeamValue::try_new(1000).unwrap(), TeamValue::try_new(1000).unwrap());
    }

    #[test]
    fn inducement_purchase_total_cost() {
        let p = InducementPurchase {
            uid: InducementId("BRIBE".to_string()),
            qty: InducementQty::try_new(2).unwrap(),
            unit_cost: InducementCost::try_new(50).unwrap(),
        };
        assert_eq!(p.total_cost(), 100); // 2 × 50 kPo
    }

    #[test]
    fn d3roll_accepte_1_2_3() {
        assert!(D3Roll::try_new(1).is_ok());
        assert!(D3Roll::try_new(2).is_ok());
        assert!(D3Roll::try_new(3).is_ok());
        assert_eq!(D3Roll::try_new(2).unwrap().value(), 2);
    }

    #[test]
    fn d3roll_rejette_0_et_4() {
        assert!(D3Roll::try_new(0).is_err());
        assert!(D3Roll::try_new(4).is_err());
        assert!(D3Roll::try_new(255).is_err());
    }

    #[test]
    fn turn_number_accepts_1_and_16() {
        assert!(TurnNumber::try_new(1).is_ok());
        assert!(TurnNumber::try_new(16).is_ok());
        assert_eq!(TurnNumber::try_new(8).unwrap().value(), 8);
    }

    #[test]
    fn turn_number_rejects_0() {
        assert_eq!(TurnNumber::try_new(0).unwrap_err(), DomainError::InvalidTurn(0));
    }

    #[test]
    fn turn_number_rejects_17() {
        assert_eq!(TurnNumber::try_new(17).unwrap_err(), DomainError::InvalidTurn(17));
    }
}
