use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::common_types::RosterId;
use crate::app::team_creation::domain::roster::RosterName;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RosterDefinition {
    pub id: RosterId,
    pub name: RosterName
}