use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitedCoach {
    pub id: String,
    pub coach_name: String,
    pub initials: String,
}

fn default_access_mode() -> String {
    "invitation".to_string()
}
fn default_notify_by_email() -> bool {
    true
}
fn default_requires_validation() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionInvitations {
    #[serde(default = "default_access_mode")]
    pub access_mode: String,
    #[serde(default = "default_requires_validation")]
    pub requires_validation: bool,
    #[serde(default)]
    pub invited_coaches: Vec<InvitedCoach>,
    pub max_participants: Option<u32>,
    pub registration_deadline: Option<String>,
    #[serde(default = "default_notify_by_email")]
    pub notify_by_email: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_validation_defaults_to_true_when_absent() {
        let json = r#"{"access_mode":"invitation","max_participants":null,"registration_deadline":null}"#;
        let inv: CompetitionInvitations = serde_json::from_str(json).unwrap();
        assert!(inv.requires_validation);
    }

    #[test]
    fn requires_validation_round_trips_explicit_false() {
        let json = r#"{"access_mode":"open","requires_validation":false,"max_participants":null,"registration_deadline":null}"#;
        let inv: CompetitionInvitations = serde_json::from_str(json).unwrap();
        assert!(!inv.requires_validation);

        let serialized = serde_json::to_string(&inv).unwrap();
        let round_tripped: CompetitionInvitations = serde_json::from_str(&serialized).unwrap();
        assert!(!round_tripped.requires_validation);
    }
}
