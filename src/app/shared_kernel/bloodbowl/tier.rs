use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Le nom d'un tier — son propre type, cf. [`SeasonName`].
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct TierName(String);

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
