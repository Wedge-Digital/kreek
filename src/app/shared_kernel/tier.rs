use crate::app::shared_kernel::name_vo::NameVo;
use nutype::nutype;
use serde::{Deserialize, Serialize};

pub type TierName = NameVo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBudget(pub u32);

impl std::fmt::Display for CreationBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[nutype(
    validate(less_or_equal = 199),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct StartingXp(u32);
