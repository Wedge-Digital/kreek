use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::identity::space_name::SpaceName;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpaceDefinition {
    pub id: SpaceId,
    pub name: SpaceName,
}
