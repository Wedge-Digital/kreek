use serde::{Deserialize, Serialize};

pub mod path {
    pub const NEW_SPACE: &str = "/app/space/create";
    pub const SPACE_ALL: &str = "/app/space/all";
    pub const SPACES_SIDEBAR: &str = "/app/spaces";
    pub const SPACE_JOIN: &str = "/app/space/join";
    pub const COACH_SELECT_WIDGET: &str = "/app/coaches/widget/select";
    pub const COACH_SEARCH_WIDGET: &str = "/app/coaches/widget/search";
    pub const COACH_SEARCH_RESULT: &str = "/app/coaches/widget/search/results";
    pub const SPACE_MEMBERS_WIDGET: &str = "/app/{space_id}/members-widget";
    pub const SPACE_ADMIN: &str = "/app/{space_id}/admin";
    pub const SPACE_ADMIN_MEMBERS_WIDGET: &str = "/app/{space_id}/admin/widgets/members";
    pub const SPACE_ADMIN_MEMBER_ROLE: &str = "/app/{space_id}/admin/members/{coach_id}/role";
    pub const SPACE_ADMIN_MEMBER_REMOVE: &str = "/app/{space_id}/admin/members/{coach_id}/remove";
    pub const SPACE_ADMIN_CANDIDATES_WIDGET: &str = "/app/{space_id}/admin/widgets/candidates";
    pub const SPACE_ADMIN_MEMBER_ADD: &str = "/app/{space_id}/admin/members/add";
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Routes;

impl Routes {
    pub fn register_space(&self) -> &'static str {
        path::NEW_SPACE
    }
    pub fn space_all(&self) -> &'static str {
        path::SPACE_ALL
    }
    pub fn join(&self) -> &'static str {
        path::SPACE_JOIN
    }
    pub fn sidebar(&self) -> &'static str {
        path::SPACES_SIDEBAR
    }

    pub fn coach_select_widget(&self) -> &'static str {
        path::COACH_SELECT_WIDGET
    }
    pub fn coach_search_widget(&self) -> &'static str {
        path::COACH_SEARCH_WIDGET
    }
    pub fn coach_search_results(&self) -> &'static str {
        path::COACH_SEARCH_RESULT
    }
    pub fn space_admin(&self, space_id: &str) -> String {
        path::SPACE_ADMIN.replace("{space_id}", space_id)
    }
    pub fn space_admin_members_widget(&self, space_id: &str) -> String {
        path::SPACE_ADMIN_MEMBERS_WIDGET.replace("{space_id}", space_id)
    }
    pub fn space_admin_candidates_widget(&self, space_id: &str) -> String {
        path::SPACE_ADMIN_CANDIDATES_WIDGET.replace("{space_id}", space_id)
    }
    pub fn space_admin_member_add(&self, space_id: &str) -> String {
        path::SPACE_ADMIN_MEMBER_ADD.replace("{space_id}", space_id)
    }
    pub fn space_admin_member_role(&self, space_id: &str, coach_id: &str) -> String {
        path::SPACE_ADMIN_MEMBER_ROLE
            .replace("{space_id}", space_id)
            .replace("{coach_id}", coach_id)
    }
    pub fn space_admin_member_remove(&self, space_id: &str, coach_id: &str) -> String {
        path::SPACE_ADMIN_MEMBER_REMOVE
            .replace("{space_id}", space_id)
            .replace("{coach_id}", coach_id)
    }
    pub fn members_widget(&self, space_id: &str) -> String {
        path::SPACE_MEMBERS_WIDGET.replace("{space_id}", space_id)
    }
}
