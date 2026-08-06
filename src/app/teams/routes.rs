pub mod path {
    pub const TEAM_DETAIL: &str = "/app/{space_id}/teams/{team_id}";
    pub const DISMISS_TEAM: &str = "/app/{space_id}/teams/{team_id}/dismiss";
    pub const PENDING_ENROLLMENT_WIDGET: &str = "/app/{space_id}/team/widgets/pending";
    pub const ENROLLED_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/enrolled";
    pub const MY_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/my-teams";
    pub const APPROVE_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/approve";
    pub const REJECT_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/reject";
    pub const DISMISS_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/dismiss";
    pub const APPROVE_ALL_ENROLLMENTS: &str = "/app/{space_id}/team/widgets/pending/approve-all";
    pub const COMPETITION_TEAMS_WIDGET: &str = "/app/team/widgets/competition-teams";
    pub const TEAM_SELECTION_WIDGET: &str = "/app/{space_id}/team/widgets/team-selection";
    pub const TEAM_SELECTION_JSON: &str = "/app/{space_id}/team/widgets/team-selection/json";
    pub const TEAM_SELECTION_TESTER: &str = "/team/widgets/tester";
    pub const TEAM_MATCH_CONTEXT_JSON: &str = "/app/{space_id}/team/widgets/match-context/json";
    pub const VALIDATE_IMPROVEMENT_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-improvement-phase";
    pub const VALIDATE_RECRUITMENT_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-recruitment-phase";
    pub const VALIDATE_DISMISSALS_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-dismissals-phase";

    // ── Recrutement ───────────────────────────────────────────────────────
    pub const RECRUITMENT_PAGE: &str = "/app/{space_id}/teams/{team_id}/recruitment";
    pub const RECRUITMENT_CATALOG_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/recruitment-catalog";
    pub const RECRUITMENT_CART_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/recruitment-cart";
    pub const RECRUITMENT_ADD_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/players/add";
    pub const RECRUITMENT_REMOVE_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/players/remove";
    pub const RECRUITMENT_ADD_STAFF: &str = "/app/{space_id}/teams/{team_id}/recruitment/staff/add";
    pub const RECRUITMENT_REMOVE_STAFF: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/staff/remove";

    // ── Renvois ───────────────────────────────────────────────────────────
    // `mark` / `unmark`, jamais `add` / `remove` : sur une page de renvois,
    // `players/add` se lirait « ajouter un joueur à l'équipe », l'inverse exact
    // de son effet.
    pub const DISMISSALS_PAGE: &str = "/app/{space_id}/teams/{team_id}/dismissals";
    pub const DISMISSALS_ROSTER_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/dismissals-roster";
    pub const DISMISSALS_CART_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/dismissals-cart";
    pub const DISMISSALS_MARK_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/players/mark";
    pub const DISMISSALS_UNMARK_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/players/unmark";
    pub const DISMISSALS_MARK_STAFF: &str = "/app/{space_id}/teams/{team_id}/dismissals/staff/mark";
    pub const DISMISSALS_UNMARK_STAFF: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/staff/unmark";
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
    pub fn my_teams_widget(&self, space_id: &str) -> String {
        path::MY_TEAMS_WIDGET.replace("{space_id}", space_id)
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

    pub fn team_match_context_json(&self, space_id: &str) -> String {
        path::TEAM_MATCH_CONTEXT_JSON.replace("{space_id}", space_id)
    }

    pub fn validate_improvement_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_IMPROVEMENT_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn validate_recruitment_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_RECRUITMENT_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn validate_dismissals_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_DISMISSALS_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    // ── Recrutement ───────────────────────────────────────────────────────

    pub fn recruitment_page(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_PAGE, space_id, team_id)
    }
    pub fn recruitment_catalog_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_CATALOG_WIDGET, space_id, team_id)
    }
    pub fn recruitment_cart_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_CART_WIDGET, space_id, team_id)
    }
    pub fn recruitment_add_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_ADD_PLAYER, space_id, team_id)
    }
    pub fn recruitment_remove_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_REMOVE_PLAYER, space_id, team_id)
    }
    pub fn recruitment_add_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_ADD_STAFF, space_id, team_id)
    }
    pub fn dismissals_page(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_PAGE, space_id, team_id)
    }
    pub fn dismissals_roster_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_ROSTER_WIDGET, space_id, team_id)
    }
    pub fn dismissals_cart_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_CART_WIDGET, space_id, team_id)
    }
    pub fn dismissals_mark_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_MARK_PLAYER, space_id, team_id)
    }
    pub fn dismissals_unmark_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_UNMARK_PLAYER, space_id, team_id)
    }
    pub fn dismissals_mark_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_MARK_STAFF, space_id, team_id)
    }
    pub fn dismissals_unmark_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_UNMARK_STAFF, space_id, team_id)
    }

    pub fn recruitment_remove_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_REMOVE_STAFF, space_id, team_id)
    }
}

fn pour(gabarit: &str, space_id: &str, team_id: &str) -> String {
    gabarit
        .replace("{space_id}", space_id)
        .replace("{team_id}", team_id)
}
