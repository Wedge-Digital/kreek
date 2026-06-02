/// Path constants — use in Rust code: `path::LOGIN`, router definitions, `HX-Redirect` headers.
pub mod path {
    pub const APP_HOME: &str = "/app/{space_id}/home";
    pub const APP_NEW_ARTICLE: &str = "/app/{space_id}/home/articles/new";
    pub const APP_POST_ARTICLE: &str = "/app/{space_id}/home/articles";
    pub const APP_ARTICLE: &str = "/app/{space_id}/home/articles/{article_id}";
    pub const APP_POST_COMMENT: &str = "/app/{space_id}/home/articles/{article_id}/comments";
}

/// Struct exposing route helpers to Askama templates.
///
/// Add `routes: Routes` to a template struct and call `{{ routes.login() }}` in the template.
/// `Routes` is `Copy + Default` — `..Default::default()` initialises it for free.
#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn space_home(&self, id: &str) -> String {
        path::APP_HOME.replace("{space_id}", id)
    }
    pub fn new_article(&self, id: &str) -> String {
        path::APP_NEW_ARTICLE.replace("{space_id}", id)
    }
    pub fn post_article(&self, id: &str) -> String {
        path::APP_POST_ARTICLE.replace("{space_id}", id)
    }
    pub fn article(&self, space_id: &str, article_id: &str) -> String {
        path::APP_ARTICLE
            .replace("{space_id}", space_id)
            .replace("{article_id}", article_id)
    }
    pub fn post_comment(&self, space_id: &str, article_id: &str) -> String {
        path::APP_POST_COMMENT
            .replace("{space_id}", space_id)
            .replace("{article_id}", article_id)
    }
}
