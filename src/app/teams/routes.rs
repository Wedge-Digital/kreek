pub mod path {
    pub const TEAM_DETAIL: &str = "/app/{space_id}/teams/{team_id}";
    pub const DISMISS_TEAM: &str = "/app/{space_id}/teams/{team_id}/dismiss";
    pub const PENDING_ENROLLMENT_WIDGET: &str = "/app/{space_id}/team/widgets/pending";
    pub const ENROLLED_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/enrolled";
    pub const APPROVE_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/approve";
    pub const REJECT_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/reject";
    pub const DISMISS_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/dismiss";
    pub const APPROVE_ALL_ENROLLMENTS: &str = "/app/{space_id}/team/widgets/pending/approve-all";
    pub const COMPETITION_TEAMS_WIDGET: &str = "/app/team/widgets/competition-teams";
    pub const TEAM_SELECTION_WIDGET: &str = "/app/{space_id}/team/widgets/team-selection";
    pub const TEAM_SELECTION_JSON: &str = "/app/{space_id}/team/widgets/team-selection/json";
    pub const TEAM_SELECTION_TESTER: &str = "/team/widgets/tester";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn team_detail(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_DETAIL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn dismiss_team(&self, space_id: &str, team_id: &str) -> String {
        path::DISMISS_TEAM
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn pending_enrollment_widget(&self, space_id: &str) -> String {
        path::PENDING_ENROLLMENT_WIDGET.replace("{space_id}", space_id)
    }
    pub fn enrolled_teams_widget(&self, space_id: &str) -> String {
        path::ENROLLED_TEAMS_WIDGET.replace("{space_id}", space_id)
    }
    pub fn approve_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::APPROVE_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn reject_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::REJECT_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn dismiss_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::DISMISS_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn approve_all_enrollments(&self) -> String {
        path::APPROVE_ALL_ENROLLMENTS.replace("{space_id}", "_")
    }
    pub fn competition_teams_widget(&self) -> String {
        path::COMPETITION_TEAMS_WIDGET.to_string()
    }
    pub fn team_selection_widget(&self, space_id: &str) -> String {
        path::TEAM_SELECTION_WIDGET.replace("{space_id}", space_id)
    }
    pub fn team_selection_json(&self, space_id: &str) -> String {
        path::TEAM_SELECTION_JSON.replace("{space_id}", space_id)
    }
}
