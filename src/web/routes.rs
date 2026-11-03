/// Path constants — use in Rust code: `path::LOGIN`, router definitions, `HX-Redirect` headers.
pub mod path {
    pub const APP_LAYOUT: &str = "/app";
    pub const MENU: &str = "/app/menu";
    /// Le flux SSE de l'onglet (carte 642) — sous `/app`, donc sous `require_auth`.
    pub const REALTIME_STREAM: &str = "/app/realtime/stream";
    /// Un onglet déclare le match qu'il regarde, ou aucun (carte 644).
    pub const REALTIME_PRESENCE: &str = "/app/realtime/presence";
    /// Ferme les flux de l'appelant (carte 646) — **réservée aux e2e** : 404
    /// hors `bypass_auth`.
    pub const REALTIME_TEST_CLOSE_STREAMS: &str = "/app/realtime/test/close-my-streams";
    /// Le widget « qui suit, qui saisit » d'un match (carte 644) — sous
    /// `space_scope`, qui résout `match_report_id`.
    pub const REALTIME_MATCH_PRESENCE: &str =
        "/app/{space_id}/realtime/matches/{match_report_id}/presence";
    pub const SPACES_WIDGET_TESTER: &str = "/spaces/widget/tester";
    pub const KREEK_SELECT_TESTER: &str = "/kreek-select-tester";
    pub const KREEK_SELECT_TESTER_DATA: &str = "/kreek-select-tester/data";
    pub const KREEK_SELECT_TESTER_COLORS: &str = "/kreek-select-tester/colors";
}

/// Struct exposing route helpers to Askama templates.
///
/// Add `routes: Routes` to a template struct and call `{{ routes.login() }}` in the template.
/// `Routes` is `Copy + Default` — `..Default::default()` initialises it for free.
#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn app_layout(&self) -> &'static str {
        path::APP_LAYOUT
    }
    pub fn home(&self) -> &'static str {
        path::APP_LAYOUT
    }
    pub fn menu(&self) -> &'static str {
        path::MENU
    }
    pub fn realtime_stream(&self) -> &'static str {
        path::REALTIME_STREAM
    }
    pub fn realtime_presence(&self) -> &'static str {
        path::REALTIME_PRESENCE
    }
    pub fn realtime_match_presence(&self, space_id: &str, match_report_id: &str) -> String {
        path::REALTIME_MATCH_PRESENCE
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }
}
