/// Path constants — use in Rust code: `path::LOGIN`, router definitions, `HX-Redirect` headers.
pub mod path {
    pub const APP_LAYOUT: &str = "/app";
    pub const SPACES: &str = "/app/spaces";
    pub const MENU: &str = "/app/menu";
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
    pub fn spaces(&self) -> &'static str {
        path::SPACES
    }
    pub fn menu(&self) -> &'static str {
        path::MENU
    }
}
