use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaffId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffName(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StaffPrice(pub u32);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StaffMaxQuantity(pub u8);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaffKind {
    Apothecary,
    Cheerleaders,
    CoachAssistant,
}

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