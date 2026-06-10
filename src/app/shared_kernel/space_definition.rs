use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::common_types::SpaceId;
use crate::app::shared_kernel::space_name::SpaceName;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpaceDefinition {
    pub id: SpaceId,
    pub name: SpaceName
}