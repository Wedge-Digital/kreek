use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId};

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub id:     CoachId,
    pub name:   CoachName,
    pub icon:   CloudinaryImage,
    pub email:  Email,
}

impl User {
    pub fn new(id: CoachId, name: CoachName, logo: CloudinaryImage, email: Email) -> Self {
        Self { id, name, icon: logo, email }
    }
}