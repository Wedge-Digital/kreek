use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitedCoach {
    pub id:         String,
    pub coach_name: String,
    pub initials:   String,
}

fn default_access_mode() -> String { "invitation".to_string() }
fn default_notify_by_email() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionInvitations {
    #[serde(default = "default_access_mode")]
    pub access_mode:            String,
    #[serde(default)]
    pub invited_coaches:        Vec<InvitedCoach>,
    pub max_participants:       Option<u32>,
    pub registration_deadline:  Option<String>,
    #[serde(default = "default_notify_by_email")]
    pub notify_by_email:        bool,
}