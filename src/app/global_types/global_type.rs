use crate::app::global_types::sulid::SUlid;

pub type EntityId = SUlid;

pub trait Entity:PartialEq<Self> {
    fn get_id(&self) -> EntityId;

    fn get_created_by(&self) -> EntityId;

    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}