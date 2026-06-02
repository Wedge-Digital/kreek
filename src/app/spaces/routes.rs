use serde::{Deserialize, Serialize};

pub mod path {
    pub const NEW_SPACE: &str = "/app/space/create";
    pub const SPACE_ALL: &str = "/app/space/all";
    pub const SPACE_JOIN: &str = "/app/space/join";
    pub const SPACE_COACH_SEARCH_WIDGET: &str = "/app/{space_id}/coaches/search-widget";
    pub const SPACE_COACH_SEARCH: &str = "/app/{space_id}/coaches/search";
    pub const SPACE_MEMBERS_WIDGET: &str = "/app/{space_id}/members-widget";
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

    pub fn coach_search_widget(&self, space_id: &str) -> String {
        path::SPACE_COACH_SEARCH_WIDGET.replace("{space_id}", space_id)
    }
    pub fn coach_search(&self, space_id: &str) -> String {
        path::SPACE_COACH_SEARCH.replace("{space_id}", space_id)
    }
    pub fn members_widget(&self, space_id: &str) -> String {
        path::SPACE_MEMBERS_WIDGET.replace("{space_id}", space_id)
    }
}
