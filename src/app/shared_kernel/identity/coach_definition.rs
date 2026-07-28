use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoachDefinition {
    pub id: CoachId,
    pub name: CoachName,
    pub icon: Option<CoachIcon>,
    pub initials: String
}