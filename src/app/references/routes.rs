pub mod path {
    pub const LEAGUE_SELECTOR: &str = "/références/leagues/selector";
    pub const SKILL_PICKER: &str = "/références/roster-lines/skill-picker";
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
}
