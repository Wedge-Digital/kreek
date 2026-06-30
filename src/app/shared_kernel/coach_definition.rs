use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::CoachId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoachDefinition {
    pub id: CoachId,
    pub name: CoachName,
    pub icon: Option<CoachIcon>,
    pub initials: String
}