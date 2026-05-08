use crate::app::shared_kernel::authorization::Authorization::SimpleUser;
use crate::app::auth::domain::coach_name::CoachName;
use crate::app::auth::domain::email::Email;
use crate::app::shared_kernel::authorization::Authorization;
use crate::app::shared_kernel::common_types::UserId;

pub struct User {
    pub id:            UserId,
    pub coach_name:    CoachName,
    pub email:         Email,
    pub password_hash: String,
}

impl User {
    pub fn new(id: UserId, coach_name: CoachName, email: Email, password_hash: String) -> Self {
        User { id, coach_name, email, password_hash}
    }
}
