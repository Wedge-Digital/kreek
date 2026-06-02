use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::sulid::SUlid;

pub type Token = SUlid;

pub struct ResetToken {
    pub token: Token,
    pub coach_name: CoachName,
    pub created_at: time::OffsetDateTime,
}
