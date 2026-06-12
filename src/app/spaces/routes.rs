use serde::{Deserialize, Serialize};

pub mod path {
    pub const NEW_SPACE: &str = "/app/space/create";
    pub const SPACE_ALL: &str = "/app/space/all";
    pub const SPACE_JOIN: &str = "/app/space/join";
    pub const COACH_SELECT_WIDGET: &str = "/app/coaches/widget/select";
    pub const COACH_SEARCH_WIDGET: &str = "/app/coaches/widget/search";
    pub const COACH_SEARCH_RESULT: &str = "/app/coaches/widget/search/results";
    pub const SPACE_MEMBERS_WIDGET: &str = "/app/{space_id}/members-widget";
    pub const SPACE_WIDGET_TESTER: &str = "/spaces/widget/tester";
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

    pub fn coach_select_widget(&self) -> &'static str { path::COACH_SELECT_WIDGET }
    pub fn coach_search_widget(&self) -> &'static str { path::COACH_SEARCH_WIDGET }
    pub fn coach_search_results(&self) -> &'static str { path::COACH_SEARCH_RESULT }
    pub fn members_widget(&self, space_id: &str) -> String {
        path::SPACE_MEMBERS_WIDGET.replace("{space_id}", space_id)
    }
    pub fn widget_tester(&self) -> &'static str {
        path::SPACE_WIDGET_TESTER
    }
}
