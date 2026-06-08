use serde::{Deserialize, Serialize};
use crate::app::team_creation::domain::roster::{RosterId, RosterName};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RosterDefinition {
    pub id: RosterId,
    pub name: RosterName
}