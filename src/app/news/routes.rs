/// Path constants — use in Rust code: `path::LOGIN`, router definitions, `HX-Redirect` headers.
pub mod path {
    pub const APP_HOME: &str  = "/app/{space_id}/home";
}

/// Struct exposing route helpers to Askama templates.
///
/// Add `routes: Routes` to a template struct and call `{{ routes.login() }}` in the template.
/// `Routes` is `Copy + Default` — `..Default::default()` initialises it for free.
#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn space_home(&self, id: &str)       -> String       { path::APP_HOME.replace("{space_id}", id) }
}