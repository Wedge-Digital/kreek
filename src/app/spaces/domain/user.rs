use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId};
use crate::app::shared_kernel::email::Email;

#[derive(Debug, Clone)]
pub struct User {
    pub id: CoachId,
    pub name: CoachName,
    pub icon: Option<CloudinaryImage>,
    pub email: Email,
}

impl User {
    pub fn new(id: CoachId, name: CoachName, logo: Option<CloudinaryImage>, email: Email) -> Self {
        Self {
            id,
            name,
            icon: logo,
            email,
        }
    }
}
