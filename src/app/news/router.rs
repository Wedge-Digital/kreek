use crate::state::AppState;
use axum::{routing::get, Router};
use crate::app::news::io::web::news_feed::get_news_feed;
use crate::app::news::routes::path;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::APP_HOME,        get(get_news_feed))
}