use crate::app::shared_kernel::tier::{CreationBudget, TierName};
use crate::app::team_creation::domain::error::DomainError;
use crate::app::shared_kernel::common_types::RosterId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RulesetId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesetName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TierId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterTier {
    pub id: TierId,
    pub name: TierName,
    pub roster_ids: Vec<RosterId>,
    pub budget: CreationBudget,
}

impl RosterTier {
    pub fn contains(&self, roster_id: &RosterId) -> bool {
        self.roster_ids.contains(roster_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub id: RulesetId,
    pub name: RulesetName,
    pub tiers: Vec<RosterTier>,
}

impl Ruleset {
    pub fn is_roster_allowed(&self, roster_id: &RosterId) -> bool {
        self.tiers.iter().any(|tier| tier.contains(roster_id))
    }

    pub fn is_roster_not_allowed(&self, roster_id: &RosterId) -> bool {
        !self.is_roster_allowed(roster_id)
    }

    pub fn get_creation_budget(&self, roster_id: &RosterId) -> Result<CreationBudget, DomainError> {
        self.tiers
            .iter()
            .find(|tier| tier.contains(roster_id))
            .map(|tier| tier.budget)
            .ok_or(DomainError::RosterNotInRuleset)
    }
}
