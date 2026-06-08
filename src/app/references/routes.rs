pub mod path {
    pub const LEAGUE_SELECTOR:       &str = "/references/leagues/selector";
    pub const SPECIAL_RULE_SELECTOR: &str = "/references/special-rules/selector";
    pub const SKILL_PICKER:          &str = "/references/roster-lines/skill-picker";
    pub const REF_WIDGETS:           &str = "/test/references/widgets";
    pub const ROSTER_PICKER:         &str = "/references/roster-picker";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn league_selector(&self, selected: &str, on_select: &str) -> String {
        format!(
            "{}?selected={}&on_select={}",
            path::LEAGUE_SELECTOR,
            selected,
            on_select,
        )
    }

    pub fn league_selector_for_roster(&self, selected: &str, on_select: &str, roster_id: &str) -> String {
        format!(
            "{}?selected={}&on_select={}&roster_id={}",
            path::LEAGUE_SELECTOR,
            selected,
            on_select,
            roster_id,
        )
    }

    pub fn special_rule_selector_for_roster(&self, selected: &str, on_select: &str, roster_id: &str) -> String {
        format!(
            "{}?selected={}&on_select={}&roster_id={}",
            path::SPECIAL_RULE_SELECTOR,
            selected,
            on_select,
            roster_id,
        )
    }

    pub fn skill_picker_base(&self) -> &'static str {
        path::SKILL_PICKER
    }

    pub fn roster_picker(&self) -> &'static str {
        path::ROSTER_PICKER
    }
}
