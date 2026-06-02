use crate::app::news::routes::Routes as NewsRoutes;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template, Default)]
#[template(path = "new-article.html")]
pub struct NewArticleTemplate {
    pub web_routes: WebRoutes,
    pub news_routes: NewsRoutes,
    pub space_id: String,
    pub image_url_value: String,
    pub image_error: Option<String>,
    pub title_value: String,
    pub tag_value: String,
    pub abstract_value: String,
    pub title_error: Option<String>,
    pub content_error: Option<String>,
}

impl IntoResponse for NewArticleTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_article(Path(space_id): Path<String>) -> impl IntoResponse {
    NewArticleTemplate {
        space_id,
        ..Default::default()
    }
    .into_response()
}
