use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::coach_icon::CoachIcon;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::UserId;

#[derive(Debug, Clone)]
pub struct User {
    pub id:            UserId,
    pub coach_name:    CoachName,
    pub coach_icon:    Option<CoachIcon>,
    pub email:         Email,
    pub password_hash: String,
}

impl axum_login::AuthUser for User {
    type Id = String;
    fn id(&self) -> Self::Id { self.id.to_string() }
    fn session_auth_hash(&self) -> &[u8] { self.password_hash.as_bytes() }
}

impl User {
    pub fn new(
        id: UserId,
        coach_name: CoachName,
        coach_icon: Option<CoachIcon>,
        email: Email,
        password_hash: String) -> Self {
        User { id, coach_name, coach_icon, email, password_hash}
    }
}
