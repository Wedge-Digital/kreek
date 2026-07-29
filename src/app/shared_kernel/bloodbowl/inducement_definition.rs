use derive_more::Display;
use nutype::nutype;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
pub struct InducementId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub struct InducementName(pub String);

#[nutype(
    validate(greater_or_equal = 0),
    derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display)
)]
pub struct InducementCost(u32);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InducementDefinition {
    pub id: InducementId,
    pub name: InducementName,
    pub cost: InducementCost,
}
