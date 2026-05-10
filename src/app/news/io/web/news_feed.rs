use askama::Template;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::news::routes::Routes as NewsRoutes;
use crate::web::app_layout::AppLayout;

#[derive(Template, Default)]
#[template(path = "news-feed.html")]
pub struct NewsFeedTemplate {
    pub routes:   NewsRoutes,
    pub space_id: String,
}

impl IntoResponse for NewsFeedTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_news_feed(Path(space_id): Path<String>, headers: HeaderMap) -> impl IntoResponse {
    let tmpl = NewsFeedTemplate { space_id, ..Default::default() };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}