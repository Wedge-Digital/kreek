use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::news::routes::Routes as NewsRoutes;
use crate::web::app_layout::AppLayout;

#[derive(Template, Default)]
#[template(path = "news-feed.html")]
pub struct NewsFeedTemplate {
    pub routes:         NewsRoutes,
}

impl IntoResponse for NewsFeedTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}


pub async fn get_news_feed(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        NewsFeedTemplate::default().into_response()
    } else {
        let content = NewsFeedTemplate::default().render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}