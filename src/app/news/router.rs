use crate::app::news::io::web::article_detail::get_article;
use crate::app::news::io::web::new_article::get_new_article;
use crate::app::news::io::web::news_feed::get_news_feed;
use crate::app::news::io::web::post_comment::post_comment;
use crate::app::news::io::web::post_new_article::post_new_article;
use crate::app::news::routes::path;
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::APP_HOME, get(get_news_feed))
        .route(path::APP_NEW_ARTICLE, get(get_new_article))
        .route(path::APP_POST_ARTICLE, post(post_new_article))
        .route(path::APP_ARTICLE, get(get_article))
        .route(path::APP_POST_COMMENT, post(post_comment))
}
