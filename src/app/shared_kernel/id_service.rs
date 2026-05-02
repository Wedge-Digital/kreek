use crate::app::shared_kernel::sulid::SUlid;
use crate::app::shared_kernel::common_types::EntityId;

pub trait IdService {
    fn generate_id(&self) -> EntityId;
}

pub struct EntityIdService {}

impl IdService for EntityIdService {
    fn generate_id(&self) -> SUlid {
        SUlid::new()
    }
}


pub struct FakeIdService {
    pub id: EntityId,
}

impl FakeIdService {
    pub fn new() -> Self {
        FakeIdService{
            id:SUlid::from_string("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap()
        }
    }

}

impl IdService for FakeIdService {
    fn generate_id(&self) -> EntityId {
        self.id.clone()
    }
}