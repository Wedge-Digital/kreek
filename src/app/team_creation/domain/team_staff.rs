use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::staff::{StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStaff {
    pub id: StaffId,
    pub name: StaffName,
    pub price: StaffPrice,
    pub max_quantity: StaffMaxQuantity,
    pub kind: StaffKind,
}

impl PartialEq for TeamStaff {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}