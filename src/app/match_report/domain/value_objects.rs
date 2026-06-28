use crate::app::match_report::domain::error::DomainError;
use crate::app::shared_kernel::inducement_definition::InducementId;
use nutype::nutype;
use serde::{Deserialize, Serialize};

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
}
