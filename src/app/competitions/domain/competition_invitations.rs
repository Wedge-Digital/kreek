use crate::app::shared_kernel::coach_initials::CoachInitials;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::CoachId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequiresValidation(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyByEmail(pub bool);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitedCoach {
    pub id: CoachId,
    pub coach_name: CoachName,
    pub initials: CoachInitials,
}

fn default_notify_by_email() -> NotifyByEmail { NotifyByEmail(true) }
fn default_requires_validation() -> RequiresValidation { RequiresValidation(true) }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccessMode {
    Invitation,
    Open,
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::Invitation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionInvitations {
    #[serde(default)]
    pub access_mode: AccessMode,
    #[serde(default = "default_requires_validation")]
    pub requires_validation: RequiresValidation,
    #[serde(default)]
    pub invited_coaches: Vec<InvitedCoach>,
    pub max_participants: Option<u32>,
    pub registration_deadline: Option<String>,
    #[serde(default = "default_notify_by_email")]
    pub notify_by_email: NotifyByEmail,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_validation_defaults_to_true_when_absent() {
        let json = r#"{"access_mode":"invitation","max_participants":null,"registration_deadline":null}"#;
        let inv: CompetitionInvitations = serde_json::from_str(json).unwrap();
        assert!(inv.requires_validation.0);
    }

    #[test]
    fn requires_validation_round_trips_explicit_false() {
        let json = r#"{"access_mode":"open","requires_validation":false,"max_participants":null,"registration_deadline":null}"#;
        let inv: CompetitionInvitations = serde_json::from_str(json).unwrap();
        assert!(!inv.requires_validation.0);
        assert_eq!(inv.access_mode, AccessMode::Open);

        let serialized = serde_json::to_string(&inv).unwrap();
        let round_tripped: CompetitionInvitations = serde_json::from_str(&serialized).unwrap();
        assert!(!round_tripped.requires_validation.0);
    }

    #[test]
    fn access_mode_defaults_to_invitation() {
        let json = r#"{"max_participants":null,"registration_deadline":null}"#;
        let inv: CompetitionInvitations = serde_json::from_str(json).unwrap();
        assert_eq!(inv.access_mode, AccessMode::Invitation);
    }
}
