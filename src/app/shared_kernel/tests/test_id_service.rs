use crate::app::shared_kernel::id_service::{FakeIdService, IdService};

#[test]
pub fn assert_fake_id_service_returns_always_the_same_id() {
    let id_service = FakeIdService::new();
    let id_1 = id_service.generate_id();
    let id_2 = id_service.generate_id();
    assert_eq!(id_1, id_2);
    assert_eq!(id_1.to_string(), "01D39ZY06FGSCTVN4T2V9PKHFZ");
}
