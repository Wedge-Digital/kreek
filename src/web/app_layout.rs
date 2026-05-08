use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use crate::app::news::routes::path;
use crate::web::routes::Routes;

#[derive(Template, Default)]
#[template(path = "app-layout.html")]
pub struct AppLayout {
    pub content: String,
    pub routes: Routes,
}

impl IntoResponse for AppLayout {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn app_layout() -> impl IntoResponse {
    Redirect::to(path::APP_HOME)
}
