pub mod path {
    pub const LEAGUE_SELECTOR: &str = "/references/leagues/selector";
    pub const SKILL_PICKER: &str = "/references/roster-lines/skill-picker";
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

    pub fn skill_picker_base(&self) -> &'static str {
        path::SKILL_PICKER
    }
}
