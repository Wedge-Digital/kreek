use crate::app::shared_kernel::identity::coach_icon::CoachIcon;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::UserId;

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub coach_name: CoachName,
    pub coach_icon: Option<CoachIcon>,
    pub email: Email,
    pub password_hash: String,
}

impl User {
    pub fn new(
        id: UserId,
        coach_name: CoachName,
        coach_icon: Option<CoachIcon>,
        email: Email,
        password_hash: String,
    ) -> Self {
        User {
            id,
            coach_name,
            coach_icon,
            email,
            password_hash,
        }
    }
}
