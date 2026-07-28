use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::identity::space_name::SpaceName;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpaceDefinition {
    pub id: SpaceId,
    pub name: SpaceName
}